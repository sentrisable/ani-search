use serde::{Deserialize,Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(rename = "MediaListCollection")]
    pub media_list_collection: MediaListCollection,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListCollection {
    pub lists: Vec<List>,
    pub user: User,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub entries: Vec<Entry>,
    pub is_completed_list: bool,
    pub is_custom_list: bool,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub advanced_scores: AdvancedScores,
    pub completed_at: CompletedAt,
    pub custom_lists: Value,
    pub hidden_from_status_lists: bool,
    pub id: i64,
    pub media: Media,
    pub media_id: i64,
    pub notes: Value,
    pub priority: i64,
    pub private: bool,
    pub progress: i64,
    pub progress_volumes: Value,
    pub repeat: i64,
    pub score: i64,
    pub started_at: StartedAt,
    pub status: String,
    pub updated_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedScores {
    #[serde(rename = "Audio")]
    pub audio: i64,
    #[serde(rename = "Characters")]
    pub characters: i64,
    #[serde(rename = "Enjoyment")]
    pub enjoyment: i64,
    #[serde(rename = "Story")]
    pub story: i64,
    #[serde(rename = "Visuals")]
    pub visuals: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedAt {
    pub day: Option<i64>,
    pub month: Option<i64>,
    pub year: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub average_score: Option<i64>,
    pub banner_image: Option<String>,
    pub chapters: Value,
    pub country_of_origin: String,
    pub cover_image: CoverImage,
    pub episodes: Option<i64>,
    pub format: String,
    pub genres: Vec<String>,
    pub id: i64,
    pub is_adult: bool,
    pub next_airing_episode: Option<NextAiringEpisode>,
    pub popularity: i64,
    pub start_date: StartDate,
    pub status: String,
    pub title: Title,
    #[serde(rename = "type")]
    pub type_field: String,
    pub volumes: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverImage {
    pub extra_large: String,
    pub large: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextAiringEpisode {
    pub airing_at: i64,
    pub episode: i64,
    pub time_until_airing: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDate {
    pub day: i64,
    pub month: i64,
    pub year: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub english: Option<String>,
    pub native: String,
    pub romaji: String,
    pub user_preferred: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedAt {
    pub day: Option<i64>,
    pub month: Option<i64>,
    pub year: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub avatar: Avatar,
    pub id: i64,
    pub media_list_options: MediaListOptions,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avatar {
    pub large: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListOptions {
    pub anime_list: AnimeList,
    pub manga_list: MangaList,
    pub row_order: String,
    pub score_format: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeList {
    pub custom_lists: Vec<Value>,
    pub section_order: Vec<String>,
    pub split_completed_section_by_format: bool,
    pub theme: Theme,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub cover_images: String,
    pub theme: String,
    pub theme_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaList {
    pub custom_lists: Vec<Value>,
    pub section_order: Vec<String>,
    pub split_completed_section_by_format: bool,
    pub theme: Theme2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme2 {
    pub cover_images: String,
    pub theme: String,
    pub theme_type: String,
}
