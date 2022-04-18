use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub content: Vec<Page>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub file_path: String,
    pub title: String,
    pub page_id: String,
    pub content_type: String,
    pub labels: Vec<String>,
}
