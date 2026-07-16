use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub episode: Episode,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub episode_string: String,
    pub upload_date: UploadDate,
    pub source_urls: Vec<SourceUrl>,
    pub thumbnail: Value,
    pub notes: Value,
    pub show: Show,
    pub page_status: PageStatus,
    pub episode_info: EpisodeInfo,
    pub version_fix: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDate {
    pub hour: i64,
    pub minute: i64,
    pub year: i64,
    pub month: i64,
    pub date: i64,
    pub second: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceUrl {
    pub source_url: String,
    pub priority: f64,
    pub source_name: String,
    pub stype: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub class_name: String,
    pub streamer_id: String,
    pub downloads: Option<Downloads>,
    pub sandbox: Option<String>,
    pub fall_back: Option<String>,
    pub file_extenstion: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    pub source_name: String,
    pub download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub english_name: String,
    pub native_name: String,
    pub slug_time: Value,
    pub thumbnail: String,
    pub last_episode_info: LastEpisodeInfo,
    pub last_episode_date: LastEpisodeDate,
    #[serde(rename = "type")]
    pub type_field: String,
    pub season: Season,
    pub score: i64,
    pub aired_start: AiredStart,
    pub available_episodes: AvailableEpisodes,
    pub episode_duration: String,
    pub episode_count: Value,
    pub last_update_end: Value,
    pub character_count: Value,
    pub description: String,
    pub broadcast_interval: Value,
    pub banner: String,
    pub characters: Value,
    pub available_episodes_detail: AvailableEpisodesDetail,
    pub name_only_string: String,
    pub is_adult: bool,
    pub related_shows: Vec<RelatedShow>,
    pub related_mangas: Vec<RelatedManga>,
    pub alt_names: Vec<String>,
    pub disqus_ids: DisqusIds,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastEpisodeInfo {
    pub sub: Sub,
    pub dub: Dub,
    pub raw: Raw,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sub {
    pub episode_string: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dub {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Raw {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastEpisodeDate {
    pub sub: Sub2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sub2 {
    pub hour: i64,
    pub minute: i64,
    pub year: i64,
    pub month: i64,
    pub date: i64,
    pub second: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub quarter: String,
    pub year: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiredStart {
    pub year: i64,
    pub month: i64,
    pub date: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableEpisodes {
    pub sub: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableEpisodesDetail {
    pub sub: Vec<String>,
    pub dub: Vec<Value>,
    pub raw: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedShow {
    pub relation: String,
    pub show_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedManga {
    pub relation: String,
    pub manga_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisqusIds {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageStatus {
    #[serde(rename = "_id")]
    pub id: String,
    pub notes: Value,
    pub page_id: String,
    pub show_id: String,
    pub views: String,
    pub likes_count: String,
    pub comment_count: String,
    pub dislikes_count: String,
    pub review_count: String,
    pub user_score_count: String,
    pub user_score_total_value: f64,
    pub user_score_aver_value: f64,
    pub viewers: Viewers,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewers {
    pub first_viewers: Vec<FirstViewer>,
    pub rec_viewers: Vec<RecViewer>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstViewer {
    pub view_count: i64,
    pub last_watched_date: String,
    pub user: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecViewer {
    pub view_count: i64,
    pub last_watched_date: String,
    pub user: Option<User>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub display_name: String,
    pub picture: Option<String>,
    pub hide_me: bool,
    pub brief: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInfo {
    pub notes: Value,
    pub thumbnails: Vec<String>,
    pub vid_inforssub: VidInforssub,
    pub upload_dates: UploadDates,
    pub vid_inforsdub: Value,
    pub vid_inforsraw: Value,
    pub description: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VidInforssub {
    pub vid_resolution: i64,
    pub vid_path: String,
    pub vid_size: i64,
    pub vid_duration: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDates {
    pub sub: String,
}
