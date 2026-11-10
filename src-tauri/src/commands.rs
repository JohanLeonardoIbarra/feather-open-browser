use tauri::State;
use crate::space_manager::{SpaceManager, SpaceManagerHandle};
use crate::state::{ApiInfo, BrowserState, SpaceState, TabState};

#[tauri::command]
pub async fn get_state(state: State<'_, SpaceManagerHandle>) -> Result<BrowserState, String> {
  let mgr = state.lock().await;
  Ok(mgr.get_state())
}

#[tauri::command]
pub async fn create_tab(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  url: Option<String>,
) -> Result<TabState, String> {
  SpaceManager::create_tab(&state, &space_id, url, true).await
}

#[tauri::command]
pub async fn activate_tab(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
) -> Result<(), String> {
  SpaceManager::activate_tab(&state, &space_id, &tab_id).await
}

#[tauri::command]
pub async fn close_tab(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
) -> Result<(), String> {
  SpaceManager::close_tab(&state, &space_id, &tab_id).await
}

#[tauri::command]
pub async fn navigate(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
  url: String,
) -> Result<TabState, String> {
  SpaceManager::navigate(&state, &space_id, &tab_id, &url).await
}

#[tauri::command]
pub async fn go_back(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
) -> Result<(), String> {
  SpaceManager::go_back(&state, &space_id, &tab_id).await
}

#[tauri::command]
pub async fn go_forward(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
) -> Result<(), String> {
  SpaceManager::go_forward(&state, &space_id, &tab_id).await
}

#[tauri::command]
pub async fn reload(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
  tab_id: String,
) -> Result<(), String> {
  SpaceManager::reload(&state, &space_id, &tab_id).await
}

#[tauri::command]
pub async fn create_space(
  state: State<'_, SpaceManagerHandle>,
  name: Option<String>,
) -> Result<SpaceState, String> {
  SpaceManager::create_space(&state, name).await
}

#[tauri::command]
pub async fn activate_space(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
) -> Result<(), String> {
  SpaceManager::activate_space(&state, &space_id).await
}

#[tauri::command]
pub async fn close_space(
  state: State<'_, SpaceManagerHandle>,
  space_id: String,
) -> Result<(), String> {
  SpaceManager::close_space(&state, &space_id).await
}

#[tauri::command]
pub async fn get_api_info(state: State<'_, Option<ApiInfo>>) -> Result<Option<ApiInfo>, String> {
  Ok(state.inner().clone())
}
