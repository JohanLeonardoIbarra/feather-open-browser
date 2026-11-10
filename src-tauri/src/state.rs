use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
  pub id: String,
  pub title: String,
  pub url: String,
  pub loading: bool,
  pub can_go_back: bool,
  pub can_go_forward: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceState {
  pub id: String,
  pub name: String,
  pub kind: String,
  pub status: String,
  pub active_tab_id: Option<String>,
  pub tabs: Vec<TabState>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BrowserState {
  pub active_space_id: Option<String>,
  pub spaces: Vec<SpaceState>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiInfo {
  pub port: u16,
  pub token: String,
}
