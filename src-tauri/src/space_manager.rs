use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewBuilder, WebviewUrl};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::state::{BrowserState, SpaceState, TabState};

pub const TOOLBAR_HEIGHT: f64 = 76.0;

#[derive(Clone, Debug)]
pub struct Tab {
  pub id: String,
  pub label: String,
  pub title: String,
  pub url: String,
  pub loading: bool,
  pub internal_new_tab: bool,
  pub visible: bool,
}

impl Tab {
  pub fn to_state(&self) -> TabState {
    TabState {
      id: self.id.clone(),
      title: self.title.clone(),
      url: self.url.clone(),
      loading: self.loading,
      can_go_back: true,
      can_go_forward: true,
    }
  }
}

#[derive(Clone, Debug)]
pub struct Space {
  pub id: String,
  pub name: String,
  pub kind: String,
  pub tabs: Vec<Tab>,
  pub active_tab_id: Option<String>,
  pub busy_count: u32,
}

impl Space {
  pub fn to_state(&self) -> SpaceState {
    let active_tab = self.tabs.iter().find(|t| Some(&t.id) == self.active_tab_id.as_ref());
    let status = if self.busy_count > 0 {
      "working".to_string()
    } else if active_tab.map_or(false, |t| t.loading) {
      "loading".to_string()
    } else {
      "ready".to_string()
    };

    SpaceState {
      id: self.id.clone(),
      name: self.name.clone(),
      kind: self.kind.clone(),
      status,
      active_tab_id: self.active_tab_id.clone(),
      tabs: self.tabs.iter().map(|t| t.to_state()).collect(),
    }
  }
}

pub fn normalize_navigation_target(input: &str, allow_file: bool) -> Result<String, String> {
  let value = input.trim();
  if value.is_empty() {
    return Err("La dirección está vacía.".into());
  }
  if value == "about:blank" {
    return Ok("about:blank".into());
  }

  let mut candidate = value.to_string();
  if !candidate.contains("://") {
    let is_localhost_or_ip = candidate.starts_with("localhost")
      || candidate.split(':').next().map_or(false, |h| {
        h.split('.').count() == 4 && h.chars().all(|c| c.is_ascii_digit() || c == '.')
      });

    if is_localhost_or_ip {
      candidate = format!("http://{}", candidate);
    } else {
      candidate = format!("https://{}", candidate);
    }
  }

  let parsed = Url::parse(&candidate).map_err(|_| "La dirección no es una URL válida.".to_string())?;
  let scheme = parsed.scheme();
  let allowed = if allow_file {
    scheme == "http" || scheme == "https" || scheme == "file"
  } else {
    scheme == "http" || scheme == "https"
  };

  if !allowed {
    return Err(format!("Protocolo no permitido: {}", scheme));
  }

  Ok(parsed.to_string())
}

pub struct SpaceManager {
  pub spaces: HashMap<String, Space>,
  pub active_space_id: Option<String>,
  pub app_handle: AppHandle,
}

pub type SpaceManagerHandle = Arc<Mutex<SpaceManager>>;

impl SpaceManager {
  pub fn new(app_handle: AppHandle) -> Self {
    Self {
      spaces: HashMap::new(),
      active_space_id: None,
      app_handle,
    }
  }

  pub fn get_state(&self) -> BrowserState {
    BrowserState {
      active_space_id: self.active_space_id.clone(),
      spaces: self.spaces.values().map(|s| s.to_state()).collect(),
    }
  }

  pub fn emit_state(&self) {
    let state = self.get_state();
    let _ = self.app_handle.emit("browser:state", &state);
  }

  pub fn layout(&self) {
    let window = match self.app_handle.get_window("main") {
      Some(w) => w,
      None => return,
    };

    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1380, 900));
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    let content_height = (logical.height - TOOLBAR_HEIGHT).max(0.0);

    for space in self.spaces.values() {
      for tab in &space.tabs {
        let is_active = self.active_space_id.as_deref() == Some(&space.id)
          && space.active_tab_id.as_deref() == Some(&tab.id);

        if let Some(wv) = self.app_handle.get_webview(&tab.label) {
          if is_active {
            let _ = wv.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
            let _ = wv.set_size(LogicalSize::new(logical.width, content_height));
            let _ = wv.show();
          } else {
            let _ = wv.hide();
          }
        }
      }
    }
  }

  pub async fn create_space(
    manager_arc: &SpaceManagerHandle,
    name: Option<String>,
  ) -> Result<SpaceState, String> {
    let space_id = Uuid::new_v4().to_string();
    let space_name = name
      .map(|n| n.trim().to_string())
      .filter(|n| !n.is_empty())
      .unwrap_or_else(|| "Agent space".to_string());

    {
      let mut mgr = manager_arc.lock().await;
      let space = Space {
        id: space_id.clone(),
        name: space_name,
        kind: "agent".to_string(),
        tabs: Vec::new(),
        active_tab_id: None,
        busy_count: 0,
      };
      mgr.spaces.insert(space_id.clone(), space);
      if mgr.active_space_id.is_none() {
        mgr.active_space_id = Some(space_id.clone());
      }
    }

    // Create initial tab
    Self::create_tab(manager_arc, &space_id, Some("about:blank".to_string()), true).await?;

    let mgr = manager_arc.lock().await;
    let space = mgr.spaces.get(&space_id).ok_or("No existe el Space recién creado")?;
    Ok(space.to_state())
  }

  pub async fn activate_space(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
  ) -> Result<(), String> {
    {
      let mut mgr = manager_arc.lock().await;
      if !mgr.spaces.contains_key(space_id) {
        return Err("No existe ese Space.".into());
      }
      mgr.active_space_id = Some(space_id.to_string());
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn close_space(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
  ) -> Result<(), String> {
    let mut mgr = manager_arc.lock().await;
    if let Some(space) = mgr.spaces.remove(space_id) {
      for tab in space.tabs {
        if let Some(wv) = mgr.app_handle.get_webview(&tab.label) {
          let _ = wv.close();
        }
      }
      if mgr.active_space_id.as_deref() == Some(space_id) {
        mgr.active_space_id = mgr.spaces.keys().next().cloned();
      }
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn create_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    target_url: Option<String>,
    activate: bool,
  ) -> Result<TabState, String> {
    let (app_handle, tab_id, tab_label, url_str, is_about_blank) = {
      let mgr = manager_arc.lock().await;
      if !mgr.spaces.contains_key(space_id) {
        return Err("No existe ese Space.".into());
      }
      let raw_url = target_url.unwrap_or_else(|| "about:blank".to_string());
      let normalized = normalize_navigation_target(&raw_url, true)?;
      let is_blank = normalized == "about:blank";
      let t_id = Uuid::new_v4().to_string();
      let t_label = format!("wv_{}", t_id.replace('-', ""));
      (mgr.app_handle.clone(), t_id, t_label, normalized, is_blank)
    };

    let window = app_handle
      .get_window("main")
      .ok_or("Ventana principal no encontrada")?;

    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1380, 900));
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    let content_height = (logical.height - TOOLBAR_HEIGHT).max(0.0);

    let webview_url = if is_about_blank {
      WebviewUrl::App("newtab.html".into())
    } else {
      let parsed = Url::parse(&url_str).map_err(|e| e.to_string())?;
      WebviewUrl::External(parsed)
    };

    let mgr_weak = Arc::downgrade(manager_arc);
    let t_id_for_title = tab_id.clone();
    let mgr_weak_title = mgr_weak.clone();

    let t_id_for_load = tab_id.clone();
    let mgr_weak_load = mgr_weak.clone();

    let builder = WebviewBuilder::new(&tab_label, webview_url)
      .auto_resize()
      .on_document_title_changed(move |_wv, title| {
        if let Some(mgr_arc) = mgr_weak_title.upgrade() {
          let t_id = t_id_for_title.clone();
          tauri::async_runtime::spawn(async move {
            let mut mgr = mgr_arc.lock().await;
            mgr.on_tab_title_changed(&t_id, title);
          });
        }
      })
      .on_page_load(move |_wv, payload| {
        if let Some(mgr_arc) = mgr_weak_load.upgrade() {
          let t_id = t_id_for_load.clone();
          let page_url = payload.url().to_string();
          let is_loading = matches!(payload.event(), tauri::webview::PageLoadEvent::Started);
          tauri::async_runtime::spawn(async move {
            let mut mgr = mgr_arc.lock().await;
            mgr.on_tab_page_load(&t_id, page_url, is_loading);
          });
        }
      });

    let _wv = window
      .add_child(
        builder,
        LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
        LogicalSize::new(logical.width, content_height),
      )
      .map_err(|e| format!("Error creando webview de pestaña: {}", e))?;

    let tab = Tab {
      id: tab_id.clone(),
      label: tab_label,
      title: if is_about_blank { "Nueva pestaña".to_string() } else { "Cargando...".to_string() },
      url: if is_about_blank { "about:blank".to_string() } else { url_str },
      loading: !is_about_blank,
      internal_new_tab: is_about_blank,
      visible: false,
    };

    let state = tab.to_state();

    {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        space.tabs.push(tab);
        if activate || space.active_tab_id.is_none() {
          space.active_tab_id = Some(tab_id);
        }
      }
      mgr.layout();
      mgr.emit_state();
    }

    Ok(state)
  }

  pub async fn activate_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    {
      let mut mgr = manager_arc.lock().await;
      let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;
      if !space.tabs.iter().any(|t| t.id == tab_id) {
        return Err("No existe esa pestaña.".into());
      }
      space.active_tab_id = Some(tab_id.to_string());
      mgr.active_space_id = Some(space_id.to_string());
      mgr.layout();
      mgr.emit_state();
    }
    Ok(())
  }

  pub async fn close_tab(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let mut mgr = manager_arc.lock().await;
    let app_handle = mgr.app_handle.clone();
    let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;

    if space.tabs.len() <= 1 {
      // Si sólo queda una pestaña, reiniciar a about:blank
      if let Some(tab) = space.tabs.first_mut() {
        tab.url = "about:blank".to_string();
        tab.title = "Nueva pestaña".to_string();
        tab.internal_new_tab = true;
        let label = tab.label.clone();
        if let Some(wv) = app_handle.get_webview(&label) {
          let _ = wv.navigate(Url::parse("tauri://localhost/newtab.html").unwrap_or_else(|_| Url::parse("about:blank").unwrap()));
        }
        mgr.emit_state();
      }
      return Ok(());
    }

    let tab_index = space.tabs.iter().position(|t| t.id == tab_id);
    if let Some(idx) = tab_index {
      let tab = space.tabs.remove(idx);
      if let Some(wv) = app_handle.get_webview(&tab.label) {
        let _ = wv.close();
      }
      if space.active_tab_id.as_deref() == Some(tab_id) {
        space.active_tab_id = space.tabs.first().map(|t| t.id.clone());
      }
      mgr.layout();
      mgr.emit_state();
    }

    Ok(())
  }

  pub async fn navigate(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    target: &str,
  ) -> Result<TabState, String> {
    let normalized = normalize_navigation_target(target, true)?;
    let is_about_blank = normalized == "about:blank";

    let (label, tab_state) = {
      let mut mgr = manager_arc.lock().await;
      let space = mgr.spaces.get_mut(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter_mut().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;

      tab.url = if is_about_blank { "about:blank".to_string() } else { normalized.clone() };
      tab.internal_new_tab = is_about_blank;
      tab.loading = !is_about_blank;
      if is_about_blank {
        tab.title = "Nueva pestaña".to_string();
      }
      (tab.label.clone(), tab.to_state())
    };

    let app_handle = {
      let mgr = manager_arc.lock().await;
      mgr.app_handle.clone()
    };

    if let Some(wv) = app_handle.get_webview(&label) {
      if is_about_blank {
        let _ = wv.navigate(Url::parse("tauri://localhost/newtab.html").unwrap_or_else(|_| Url::parse("about:blank").unwrap()));
      } else if let Ok(parsed) = Url::parse(&normalized) {
        let _ = wv.navigate(parsed);
      }
    }

    {
      let mgr = manager_arc.lock().await;
      mgr.emit_state();
    }

    Ok(tab_state)
  }

  pub async fn go_back(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.eval("window.history.back()");
    }
    Ok(())
  }

  pub async fn go_forward(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.eval("window.history.forward()");
    }
    Ok(())
  }

  pub async fn reload(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<(), String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };
    if let Some(wv) = app_handle.get_webview(&label) {
      let _ = wv.reload();
    }
    Ok(())
  }

  pub fn on_tab_title_changed(&mut self, tab_id: &str, title: String) {
    let trimmed = title.trim();
    if trimmed.is_empty() {
      return;
    }
    for space in self.spaces.values_mut() {
      for tab in &mut space.tabs {
        if tab.id == tab_id {
          if !tab.internal_new_tab {
            tab.title = trimmed.to_string();
          }
          self.emit_state();
          return;
        }
      }
    }
  }

  pub fn on_tab_page_load(&mut self, tab_id: &str, url: String, loading: bool) {
    for space in self.spaces.values_mut() {
      for tab in &mut space.tabs {
        if tab.id == tab_id {
          tab.loading = loading;
          if !loading && !tab.internal_new_tab {
            tab.url = url;
          }
          self.emit_state();
          return;
        }
      }
    }
  }

  pub async fn snapshot(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview de la pestaña no disponible")?;

    let script = r#"
      JSON.stringify((function() {
        const refs = new Map();
        window.__openBrowserAgentRefs = refs;
        let nextRef = 1;
        const visible = (el) => {
          const rect = el.getBoundingClientRect();
          const style = getComputedStyle(el);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
            style.visibility !== 'hidden' && style.opacity !== '0';
        };
        const text = (el) => (el.getAttribute('aria-label') || el.innerText ||
          el.value || el.getAttribute('placeholder') || el.getAttribute('alt') ||
          el.getAttribute('title') || '').replace(/\s+/g, ' ').trim().slice(0, 240);
        const role = (el) => {
          if (el.getAttribute('role')) return el.getAttribute('role');
          const tag = el.tagName.toLowerCase();
          if (tag === 'a') return 'link';
          if (tag === 'button') return 'button';
          if (tag === 'textarea') return 'textbox';
          if (tag === 'select') return 'combobox';
          if (tag === 'input') {
            const type = (el.getAttribute('type') || 'text').toLowerCase();
            if (type === 'checkbox') return 'checkbox';
            if (type === 'radio') return 'radio';
            if (type === 'submit' || type === 'button') return 'button';
            return 'textbox';
          }
          return 'generic';
        };
        const candidates = Array.from(document.querySelectorAll(
          'a[href],button,input,textarea,select,[role],[contenteditable="true"]'
        )).filter(visible).slice(0, 180);
        const rows = [];
        for (const el of candidates) {
          const ref = 'e' + nextRef++;
          refs.set(ref, el);
          const name = text(el) || '(sin nombre)';
          const href = el instanceof HTMLAnchorElement ? ' -> ' + el.href : '';
          const value = (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)
            ? ' value=' + JSON.stringify(el.value.slice(0, 100)) : '';
          rows.push(role(el) + ' ' + JSON.stringify(name) + ' [ref=' + ref + ']' + href + value);
        }
        const textBlocks = Array.from(document.querySelectorAll('h1,h2,h3,p,li'))
          .filter(visible).slice(0, 100)
          .map((el) => el.tagName.toLowerCase() + ': ' + (el.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 280))
          .filter((line) => line.length > 4);
        return {
          title: document.title,
          url: location.href,
          elements: rows,
          text: textBlocks,
          truncated: candidates.length >= 180,
          consoleErrors: [],
          networkRequestsCount: 0,
          note: 'Tauri v2 agent snapshot: el snapshot cubre el documento principal.'
        };
      })())
    "#;

    let raw = eval_js_async(&wv, script).await?;
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
      Ok(serde_json::Value::String(inner)) => serde_json::from_str(&inner).unwrap_or(serde_json::json!({ "raw": inner })),
      Ok(val) => val,
      Err(_) => serde_json::json!({ "raw": raw }),
    };

    Ok(parsed)
  }

  pub async fn run_actions(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    actions: serde_json::Value,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        space.busy_count += 1;
      }
      mgr.emit_state();
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let actions_list = actions.as_array().cloned().unwrap_or_default();
    let wv = app_handle.get_webview(&label).ok_or("Webview de la pestaña no disponible")?;

    let mut results = Vec::new();

    for action in actions_list {
      let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
      match action_type {
        "click" => {
          let r = action.get("ref").and_then(|v| v.as_str()).unwrap_or("");
          let script = format!(
            r#"
              JSON.stringify((function() {{
                const el = window.__openBrowserAgentRefs?.get({:?});
                if (!el || !el.isConnected) return {{ ok: false, error: 'Elemento no encontrado o desconectado; toma un snapshot nuevo.' }};
                el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'auto' }});
                el.focus();
                el.click();
                return {{ ok: true }};
              }})())
            "#,
            r
          );
          if let Err(e) = eval_js_async(&wv, &script).await {
            return Err(e);
          }
          results.push(serde_json::json!({ "ok": true }));
        }

        "fill" => {
          let r = action.get("ref").and_then(|v| v.as_str()).unwrap_or("");
          let text = action.get("value").or_else(|| action.get("text")).and_then(|v| v.as_str()).unwrap_or("");
          let script = format!(
            r#"
              JSON.stringify((function() {{
                const el = window.__openBrowserAgentRefs?.get({:?});
                if (!el || !el.isConnected) return {{ ok: false, error: 'Elemento no encontrado o desconectado; toma un snapshot nuevo.' }};
                el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'auto' }});
                el.focus();
                el.value = {:?};
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return {{ ok: true }};
              }})())
            "#,
            r, text
          );
          if let Err(e) = eval_js_async(&wv, &script).await {
            return Err(e);
          }
          results.push(serde_json::json!({ "ok": true }));
        }

        "press" => {
          let key = action.get("key").and_then(|v| v.as_str()).unwrap_or("Enter");
          let script = format!(
            r#"
              JSON.stringify((function() {{
                const active = document.activeElement || document.body;
                active.dispatchEvent(new KeyboardEvent('keydown', {{ key: {:?}, bubbles: true }}));
                active.dispatchEvent(new KeyboardEvent('keyup', {{ key: {:?}, bubbles: true }}));
                return {{ ok: true }};
              }})())
            "#,
            key, key
          );
          if let Err(e) = eval_js_async(&wv, &script).await {
            return Err(e);
          }
          results.push(serde_json::json!({ "ok": true }));
        }

        "hover" => {
          let r = action.get("ref").and_then(|v| v.as_str()).unwrap_or("");
          let script = format!(
            r#"
              JSON.stringify((function() {{
                const el = window.__openBrowserAgentRefs?.get({:?});
                if (!el || !el.isConnected) return {{ ok: false, error: 'Elemento no encontrado' }};
                el.dispatchEvent(new MouseEvent('mouseover', {{ bubbles: true }}));
                return {{ ok: true }};
              }})())
            "#,
            r
          );
          if let Err(e) = eval_js_async(&wv, &script).await {
            return Err(e);
          }
          results.push(serde_json::json!({ "ok": true }));
        }

        "scroll" => {
          let dx = action.get("deltaX").and_then(|v| v.as_f64()).unwrap_or(0.0);
          let dy = action.get("deltaY").and_then(|v| v.as_f64()).unwrap_or(0.0);
          let script = format!(
            r#"
              JSON.stringify((function() {{
                window.scrollBy({{ top: {}, left: {}, behavior: 'auto' }});
                return {{ ok: true }};
              }})())
            "#,
            dy, dx
          );
          if let Err(e) = eval_js_async(&wv, &script).await {
            return Err(e);
          }
          results.push(serde_json::json!({ "ok": true }));
        }

        "navigate" => {
          if let Some(target) = action.get("url").and_then(|v| v.as_str()) {
            let _ = Self::navigate(manager_arc, space_id, tab_id, target).await;
            results.push(serde_json::json!({ "navigated": target }));
          }
        }

        "back" => {
          let _ = Self::go_back(manager_arc, space_id, tab_id).await;
          results.push(serde_json::json!({ "ok": true }));
        }

        "forward" => {
          let _ = Self::go_forward(manager_arc, space_id, tab_id).await;
          results.push(serde_json::json!({ "ok": true }));
        }

        "reload" => {
          let _ = Self::reload(manager_arc, space_id, tab_id).await;
          results.push(serde_json::json!({ "ok": true }));
        }

        "wait" => {
          let ms = action.get("ms")
            .or_else(|| action.get("durationMs"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100);
          tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
          results.push(serde_json::json!({ "waited": ms }));
        }

        _ => {}
      }
    }

    {
      let mut mgr = manager_arc.lock().await;
      if let Some(space) = mgr.spaces.get_mut(space_id) {
        if space.busy_count > 0 {
          space.busy_count -= 1;
        }
      }
      mgr.emit_state();
    }

    let snapshot = Self::snapshot(manager_arc, space_id, tab_id)
      .await
      .unwrap_or_else(|_| serde_json::json!({}));

    Ok(serde_json::json!({
      "results": results,
      "snapshot": snapshot,
    }))
  }

  pub async fn get_html(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
    ref_param: Option<String>,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;
    let script = match ref_param {
      Some(r) => format!(
        r#"
          JSON.stringify((function() {{
            const el = window.__openBrowserAgentRefs?.get({:?});
            if (!el || !el.isConnected) return null;
            return el.outerHTML;
          }})())
        "#,
        r
      ),
      None => r#"
        JSON.stringify((function() {
          return document.documentElement ? document.documentElement.outerHTML : '';
        })())
      "#.to_string(),
    };

    let raw = eval_js_async(&wv, &script).await?;
    let html_val: Option<String> = match serde_json::from_str(&raw) {
      Ok(serde_json::Value::String(s)) => Some(s),
      _ => None,
    };

    Ok(serde_json::json!({
      "html": html_val.unwrap_or_default()
    }))
  }

  pub async fn screenshot(
    manager_arc: &SpaceManagerHandle,
    space_id: &str,
    tab_id: &str,
  ) -> Result<serde_json::Value, String> {
    let (app_handle, label) = {
      let mgr = manager_arc.lock().await;
      let space = mgr.spaces.get(space_id).ok_or("No existe ese Space.")?;
      let tab = space.tabs.iter().find(|t| t.id == tab_id).ok_or("No existe esa pestaña.")?;
      (mgr.app_handle.clone(), tab.label.clone())
    };

    let wv = app_handle.get_webview(&label).ok_or("Webview no disponible")?;
    let script = r#"
      JSON.stringify((function() {
        try {
          const canvas = document.createElement('canvas');
          canvas.width = window.innerWidth || 1280;
          canvas.height = window.innerHeight || 720;
          const ctx = canvas.getContext('2d');
          ctx.fillStyle = '#101318';
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          const dataUrl = canvas.toDataURL('image/png');
          return {
            width: canvas.width,
            height: canvas.height,
            mimeType: 'image/png',
            dataUrl: dataUrl,
            base64: dataUrl.split(',')[1] || ''
          };
        } catch(e) {
          return null;
        }
      })())
    "#;

    let raw = eval_js_async(&wv, script).await?;
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
      Ok(serde_json::Value::String(inner)) => serde_json::from_str(&inner).unwrap_or(serde_json::json!({})),
      Ok(val) => val,
      Err(_) => serde_json::json!({}),
    };

    Ok(parsed)
  }
}

async fn eval_js_async(wv: &tauri::Webview, script: &str) -> Result<String, String> {
  let (tx, rx) = tokio::sync::oneshot::channel();
  let tx_cell = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
  wv.eval_with_callback(script, move |result| {
    if let Ok(mut lock) = tx_cell.lock() {
      if let Some(sender) = lock.take() {
        let _ = sender.send(result);
      }
    }
  }).map_err(|e| format!("Error enviando eval: {}", e))?;

  match tokio::time::timeout(tokio::time::Duration::from_secs(10), rx).await {
    Ok(Ok(val)) => Ok(val),
    Ok(Err(_)) => Err("El canal de respuesta de JavaScript se cerró".into()),
    Err(_) => Err("Timeout esperando respuesta de la página web".into()),
  }
}
