use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub episode: Option<Episode>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub episode_string: Option<String>,
    pub upload_date: Option<UploadDate>,
    pub source_urls: Option<Vec<SourceUrl>>,
    pub thumbnail: Option<Value>,
    pub notes: Option<Value>,
    pub show: Option<Show>,
    pub page_status: Option<PageStatus>,
    pub episode_info: Option<EpisodeInfo>,
    pub version_fix: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDate {
    pub hour: Option<i64>,
    pub minute: Option<i64>,
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub date: Option<i64>,
    pub second: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceUrl {
    pub source_url: Option<String>,
    pub priority: Option<f64>,
    pub source_name: Option<String>,
    pub stype: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub class_name: Option<String>,
    pub streamer_id: Option<String>,
    pub downloads: Option<Downloads>,
    pub sandbox: Option<String>,
    pub fall_back: Option<String>,
    pub file_extenstion: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    pub source_name: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub name: Option<String>,
    pub english_name: Option<String>,
    pub native_name: Option<String>,
    pub slug_time: Option<Value>,
    pub thumbnail: Option<String>,
    pub last_episode_info: Option<LastEpisodeInfo>,
    pub last_episode_date: Option<LastEpisodeDate>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub season: Option<Season>,
    pub score: Option<Value>,
    pub aired_start: Option<AiredStart>,
    pub available_episodes: Option<AvailableEpisodes>,
    pub episode_duration: Option<String>,
    pub episode_count: Option<Value>,
    pub last_update_end: Option<Value>,
    pub character_count: Option<Value>,
    pub description: Option<String>,
    pub broadcast_interval: Option<Value>,
    pub banner: Option<String>,
    pub characters: Option<Value>,
    pub available_episodes_detail: Option<AvailableEpisodesDetail>,
    pub name_only_string: Option<String>,
    pub is_adult: Option<bool>,
    pub related_shows: Option<Vec<RelatedShow>>,
    pub related_mangas: Option<Vec<RelatedManga>>,
    pub alt_names: Option<Vec<String>>,
    pub disqus_ids: Option<DisqusIds>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastEpisodeInfo {
    pub sub: Option<Sub>,
    pub dub: Option<Dub>,
    pub raw: Option<Raw>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sub {
    pub episode_string: Option<String>,
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
    pub sub: Option<Sub2>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sub2 {
    pub hour: Option<i64>,
    pub minute: Option<i64>,
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub date: Option<i64>,
    pub second: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub quarter: Option<String>,
    pub year: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiredStart {
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub date: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableEpisodes {
    pub sub: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableEpisodesDetail {
    pub sub: Option<Vec<String>>,
    pub dub: Option<Vec<Value>>,
    pub raw: Option<Vec<Value>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedShow {
    pub relation: Option<String>,
    pub show_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedManga {
    pub relation: Option<String>,
    pub manga_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisqusIds {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageStatus {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub notes: Option<Value>,
    pub page_id: Option<String>,
    pub show_id: Option<String>,
    pub views: Option<String>,
    pub likes_count: Option<String>,
    pub comment_count: Option<String>,
    pub dislikes_count: Option<String>,
    pub review_count: Option<String>,
    pub user_score_count: Option<String>,
    pub user_score_total_value: Option<f64>,
    pub user_score_aver_value: Option<f64>,
    pub viewers: Option<Viewers>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewers {
    pub first_viewers: Option<Vec<FirstViewer>>,
    pub rec_viewers: Option<Vec<RecViewer>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstViewer {
    pub view_count: Option<i64>,
    pub last_watched_date: Option<String>,
    pub user: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecViewer {
    pub view_count: Option<i64>,
    pub last_watched_date: Option<String>,
    pub user: Option<User>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub hide_me: Option<bool>,
    pub brief: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInfo {
    pub notes: Option<Value>,
    pub thumbnails: Option<Vec<String>>,
    pub vid_inforssub: Option<VidInforssub>,
    pub upload_dates: Option<UploadDates>,
    pub vid_inforsdub: Option<Value>,
    pub vid_inforsraw: Option<Value>,
    pub description: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VidInforssub {
    pub vid_resolution: Option<i64>,
    pub vid_path: Option<String>,
    pub vid_size: Option<i64>,
    pub vid_duration: Option<f64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDates {
    pub sub: Option<String>,
    pub dub: Option<String>,
    pub raw: Option<String>,
}
