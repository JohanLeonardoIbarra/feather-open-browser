pub mod agent_api;
pub mod commands;
pub mod space_manager;
pub mod state;
pub mod visual_agent;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use space_manager::{SpaceManager, SpaceManagerHandle};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  #[cfg(target_os = "linux")]
  {
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
      std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
  }

  tauri::Builder::default()
    .plugin(
      tauri_plugin_log::Builder::default()
        .level(log::LevelFilter::Info)
        .build(),
    )
    .invoke_handler(tauri::generate_handler![
      commands::get_state,
      commands::create_tab,
      commands::activate_tab,
      commands::close_tab,
      commands::navigate,
      commands::go_back,
      commands::go_forward,
      commands::reload,
      commands::create_space,
      commands::activate_space,
      commands::close_space,
      commands::get_api_info,
    ])
    .setup(|app| {
      let handle = app.handle().clone();
      let space_manager: SpaceManagerHandle = Arc::new(Mutex::new(SpaceManager::new(handle.clone())));
      app.manage(space_manager.clone());

      match agent_api::AgentApiServer::start(space_manager.clone()) {
        Ok(server) => {
          let info = Some(server.to_api_info());
          app.manage(info);
          app.manage(server);
        }
        Err(err) => {
          log::error!("No se pudo iniciar el Agent API Server: {}", err);
          let info: Option<state::ApiInfo> = None;
          app.manage(info);
        }
      }

      if let Some(main_window) = app.get_window("main") {
        let mgr_for_resize = space_manager.clone();
        main_window.on_window_event(move |event| {
          if let tauri::WindowEvent::Resized(_) = event {
            let mgr_arc = mgr_for_resize.clone();
            tauri::async_runtime::spawn(async move {
              let mgr = mgr_arc.lock().await;
              mgr.layout();
            });
          }
        });
      }

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
