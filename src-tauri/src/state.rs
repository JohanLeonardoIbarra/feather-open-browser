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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requested_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error_code: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub http_status: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NavigationStatus {
  pub ok: bool,
  pub requested_url: String,
  pub url: String,
  pub title: String,
  pub error_code: Option<String>,
  pub http_status: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceState {
  pub id: String,
  pub name: String,
  pub kind: String,
  pub status: String,
  pub active_tab_id: Option<String>,
  #[serde(default)]
  pub ignore_cert_errors: bool,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotOptions {
  pub selector: Option<String>,
  pub role: Option<String>,
  pub offset: Option<usize>,
  pub limit: Option<usize>,
  pub include_tables: Option<bool>,
  pub max_table_rows: Option<usize>,
  pub table_row_offset: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetHtmlOptions {
  pub ref_id: Option<String>,
  pub selector: Option<String>,
  pub max_chars: Option<usize>,
  pub offset: Option<usize>,
  pub text_only: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractOptions {
  pub selector: String,
  pub limit: Option<usize>,
  pub offset: Option<usize>,
  pub include_attributes: Option<bool>,
}
