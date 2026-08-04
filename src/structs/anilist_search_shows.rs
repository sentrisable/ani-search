use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(rename = "Page")]
    pub page: Page,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub media: Option<Vec<Medum>>,
    pub page_info: PageInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Medum {
    pub average_score: Option<i64>,
    pub banner_image: Option<String>,
    pub chapters: Option<Value>,
    pub cover_image: Option<CoverImage>,
    pub description: Option<String>,
    pub duration: Option<i64>,
    pub end_date: Option<EndDate>,
    pub episodes: Option<i64>,
    pub format: Option<String>,
    pub genres: Option<Vec<String>>,
    pub id: Option<i64>,
    pub is_adult: Option<bool>,
    pub media_list_entry: Option<MediaListEntry>,
    pub next_airing_episode: Option<NextAiringEpisode>,
    pub popularity: Option<i64>,
    pub season: Option<String>,
    pub season_year: Option<i64>,
    pub start_date: Option<StartDate>,
    pub status: Option<String>,
    pub studios: Option<Studios>,
    pub title: Option<Title>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub volumes: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverImage {
    pub color: Option<String>,
    pub extra_large: Option<String>,
    pub large: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndDate {
    pub day: Option<Value>,
    pub month: Option<Value>,
    pub year: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListEntry {
    pub id: Option<i64>,
    pub status: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextAiringEpisode {
    pub airing_at: Option<i64>,
    pub episode: Option<i64>,
    pub time_until_airing: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDate {
    pub day: Option<i64>,
    pub month: Option<i64>,
    pub year: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Studios {
    pub edges: Option<Vec<Edge>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub is_main: Option<bool>,
    pub node: Option<Node>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: Option<i64>,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub english: Option<String>,
    pub romaji: Option<String>,
    pub user_preferred: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub current_page: Option<i64>,
    pub has_next_page: Option<bool>,
    pub last_page: Option<i64>,
    pub per_page: Option<i64>,
    pub total: Option<i64>,
}
