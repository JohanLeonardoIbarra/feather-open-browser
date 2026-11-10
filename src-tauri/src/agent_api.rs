use std::fs::{create_dir_all, remove_file, set_permissions, write, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Response, Server, StatusCode};
use uuid::Uuid;

use crate::space_manager::{SpaceManager, SpaceManagerHandle};
use crate::state::ApiInfo;

pub struct AgentApiServer {
  pub host: String,
  pub port: u16,
  pub token: String,
  pub config_path: PathBuf,
}

impl AgentApiServer {
  pub fn start(manager: SpaceManagerHandle) -> Result<Self, String> {
    let server = Server::http("127.0.0.1:0").map_err(|e| format!("Error iniciando tiny_http: {}", e))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).ok_or("No se pudo obtener el puerto local")?;
    let host = "127.0.0.1".to_string();
    let token = format!("{}{}", Uuid::new_v4().to_string().replace('-', ""), Uuid::new_v4().to_string().replace('-', ""));

    let config_path = determine_config_path();
    if let Some(parent) = config_path.parent() {
      let _ = create_dir_all(parent);
    }

    let config_content = json!({
      "host": host,
      "port": port,
      "token": token,
      "protocol": "open-browser-agent-api-v1"
    });

    write(&config_path, serde_json::to_string_pretty(&config_content).unwrap())
      .map_err(|e| format!("Error escribiendo agent-api.json en {:?}: {}", config_path, e))?;

    let _ = set_permissions(&config_path, Permissions::from_mode(0o600));

    let server_arc = Arc::new(server);
    let token_clone = token.clone();
    let manager_clone = manager.clone();

    thread::spawn(move || {
      for mut request in server_arc.incoming_requests() {
        let origin = request.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("origin"));
        if origin.is_some() {
          let res = json_response(403, &json!({ "error": "Las solicitudes desde páginas web no están permitidas." }));
          let _ = request.respond(res);
          continue;
        }

        let url = request.url().to_string();
        let path = url.split('?').next().unwrap_or("").to_string();

        if request.method() == &Method::Get && path == "/health" {
          let res = json_response(200, &json!({ "ok": true, "service": "open-browser-agent-api-v1" }));
          let _ = request.respond(res);
          continue;
        }

        let auth_header = request
          .headers()
          .iter()
          .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case("authorization"))
          .map(|h| h.value.as_str().to_string())
          .unwrap_or_default();

        let supplied_token = auth_header.trim_start_matches("Bearer ").trim();
        if supplied_token != token_clone {
          let res = json_response(401, &json!({ "error": "Token ausente o incorrecto." }));
          let _ = request.respond(res);
          continue;
        }

        let handle = tauri::async_runtime::handle();
        let mgr = manager_clone.clone();

        match (request.method(), path.as_str()) {
          (&Method::Get, "/v1/spaces") => {
            let spaces = handle.block_on(async {
              let m = mgr.lock().await;
              m.get_state().spaces
            });
            let _ = request.respond(json_response(200, &json!({ "spaces": spaces })));
          }

          (&Method::Post, "/v1/spaces") => {
            let body = read_json_body(&mut request);
            let name = body.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
            let target_url = body.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());

            let result = handle.block_on(async {
              let space = SpaceManager::create_space(&mgr, name).await?;
              if let Some(target) = target_url {
                if let Some(tab_id) = &space.active_tab_id {
                  let _ = SpaceManager::navigate(&mgr, &space.id, tab_id, &target).await;
                }
              }
              let m = mgr.lock().await;
              let updated_space = m.spaces.get(&space.id).map(|s| s.to_state());
              let tab = updated_space.as_ref().and_then(|s| s.tabs.first().cloned());
              Ok::<(Value, Value), String>((json!(updated_space), json!(tab)))
            });

            match result {
              Ok((space, tab)) => {
                let _ = request.respond(json_response(201, &json!({ "space": space, "tab": tab })));
              }
              Err(err) => {
                let _ = request.respond(json_response(400, &json!({ "error": err })));
              }
            }
          }

          _ => {
            // Manejar rutas con parámetros: /v1/spaces/:id, /v1/spaces/:id/tabs, /v1/spaces/:id/tabs/:tabId/...
            let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
            if segments.len() == 3 && segments[0] == "v1" && segments[1] == "spaces" && request.method() == &Method::Delete {
              let space_id = segments[2];
              let res = handle.block_on(SpaceManager::close_space(&mgr, space_id));
              match res {
                Ok(_) => { let _ = request.respond(json_response(200, &json!({ "closed": true }))); }
                Err(err) => { let _ = request.respond(json_response(404, &json!({ "error": err }))); }
              }
            } else if segments.len() == 4 && segments[0] == "v1" && segments[1] == "spaces" && segments[3] == "tabs" {
              let space_id = segments[2];
              if request.method() == &Method::Get {
                let tabs = handle.block_on(async {
                  let m = mgr.lock().await;
                  m.spaces.get(space_id).map(|s| s.to_state().tabs)
                });
                match tabs {
                  Some(t) => { let _ = request.respond(json_response(200, &json!({ "tabs": t }))); }
                  None => { let _ = request.respond(json_response(404, &json!({ "error": "No existe ese Space." }))); }
                }
              } else if request.method() == &Method::Post {
                let body = read_json_body(&mut request);
                let target = body.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
                let res = handle.block_on(SpaceManager::create_tab(&mgr, space_id, target, true));
                match res {
                  Ok(tab) => { let _ = request.respond(json_response(201, &json!({ "tab": tab }))); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else {
                let _ = request.respond(json_response(404, &json!({ "error": "Ruta no encontrada." })));
              }
            } else if segments.len() >= 5 && segments[0] == "v1" && segments[1] == "spaces" && segments[3] == "tabs" {
              let space_id = segments[2];
              let tab_id = segments[4];
              let operation = segments.get(5).copied().unwrap_or("");

              if operation.is_empty() && request.method() == &Method::Delete {
                let res = handle.block_on(SpaceManager::close_tab(&mgr, space_id, tab_id));
                match res {
                  Ok(_) => { let _ = request.respond(json_response(200, &json!({ "closed": true }))); }
                  Err(err) => { let _ = request.respond(json_response(404, &json!({ "error": err }))); }
                }
              } else if operation == "navigate" && request.method() == &Method::Post {
                let body = read_json_body(&mut request);
                let target = body.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let res = handle.block_on(SpaceManager::navigate(&mgr, space_id, tab_id, target));
                match res {
                  Ok(tab) => { let _ = request.respond(json_response(200, &json!({ "tab": tab }))); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else if operation == "snapshot" && request.method() == &Method::Get {
                let res = handle.block_on(SpaceManager::snapshot(&mgr, space_id, tab_id));
                match res {
                  Ok(snap) => { let _ = request.respond(json_response(200, &snap)); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else if operation == "actions" && request.method() == &Method::Post {
                let body = read_json_body(&mut request);
                let actions = body.get("actions").cloned().unwrap_or(json!([]));
                let res = handle.block_on(SpaceManager::run_actions(&mgr, space_id, tab_id, actions));
                match res {
                  Ok(val) => { let _ = request.respond(json_response(200, &val)); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else if operation == "html" && request.method() == &Method::Get {
                let ref_param = parse_query_param(&url, "ref");
                let res = handle.block_on(SpaceManager::get_html(&mgr, space_id, tab_id, ref_param));
                match res {
                  Ok(val) => { let _ = request.respond(json_response(200, &val)); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else if operation == "screenshot" && request.method() == &Method::Get {
                let res = handle.block_on(SpaceManager::screenshot(&mgr, space_id, tab_id));
                match res {
                  Ok(val) => { let _ = request.respond(json_response(200, &val)); }
                  Err(err) => { let _ = request.respond(json_response(400, &json!({ "error": err }))); }
                }
              } else if operation == "console" && request.method() == &Method::Get {
                let _ = request.respond(json_response(200, &json!({ "logs": [] })));
              } else if operation == "network" && request.method() == &Method::Get {
                let _ = request.respond(json_response(200, &json!({ "requests": [] })));
              } else {
                let _ = request.respond(json_response(404, &json!({ "error": "Ruta no encontrada." })));
              }
            } else {
              let _ = request.respond(json_response(404, &json!({ "error": "Ruta no encontrada." })));
            }
          }
        }
      }
    });

    Ok(Self {
      host,
      port,
      token,
      config_path,
    })
  }

  pub fn to_api_info(&self) -> ApiInfo {
    ApiInfo {
      port: self.port,
      token: self.token.clone(),
    }
  }
}

impl Drop for AgentApiServer {
  fn drop(&mut self) {
    let _ = remove_file(&self.config_path);
  }
}

fn determine_config_path() -> PathBuf {
  if let Ok(path) = std::env::var("OPEN_BROWSER_CONFIG") {
    return PathBuf::from(path);
  }
  if let Ok(user_data) = std::env::var("OPEN_BROWSER_USER_DATA") {
    return PathBuf::from(user_data).join("agent-api.json");
  }
  let base = std::env::var("XDG_CONFIG_HOME")
    .map(PathBuf::from)
    .unwrap_or_else(|_| {
      let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
      PathBuf::from(home).join(".config")
    });
  base.join("open-browser").join("agent-api.json")
}

fn read_json_body(request: &mut tiny_http::Request) -> Value {
  let mut content = Vec::new();
  let _ = request.as_reader().read_to_end(&mut content);
  if content.is_empty() {
    return json!({});
  }
  serde_json::from_slice(&content).unwrap_or(json!({}))
}

fn json_response(status: u16, value: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
  let bytes = serde_json::to_vec(value).unwrap_or_default();
  let content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap();
  let cache_control = Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap();
  let nosniff = Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap();

  Response::new(
    StatusCode(status),
    vec![content_type, cache_control, nosniff],
    std::io::Cursor::new(bytes),
    None,
    None,
  )
}

fn parse_query_param(url: &str, param: &str) -> Option<String> {
  let query = url.split('?').nth(1)?;
  for pair in query.split('&') {
    let mut parts = pair.split('=');
    if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
      if k == param {
        return Some(v.to_string());
      }
    }
  }
  None
}
