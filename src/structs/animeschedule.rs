use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub page: i64,
    pub total_amount: i64,
    pub anime: Vec<Anime>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anime {
    pub id: String,
    pub title: String,
    pub route: String,
    pub premier: String,
    pub sub_premier: String,
    pub dub_premier: String,
    pub month: Option<String>,
    pub year: Option<i64>,
    pub season: Season,
    pub episode_override: EpisodeOverride,
    pub sub_episode_override: SubEpisodeOverride,
    pub dub_episode_override: DubEpisodeOverride,
    pub delayed_from: String,
    pub delayed_until: String,
    pub sub_delayed_from: String,
    pub sub_delayed_until: String,
    pub dub_delayed_from: String,
    pub dub_delayed_until: String,
    pub jpn_time: String,
    pub sub_time: String,
    pub dub_time: String,
    pub description: Option<String>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub studios: Vec<Studio>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub media_types: Vec<MediaType>,
    pub episodes: Option<i64>,
    pub length_min: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub image_version_route: String,
    pub stats: Stats,
    pub names: Names,
    pub websites: Websites,
    pub relations: Option<Relations>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub title: String,
    pub year: String,
    pub season: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeOverride {
    pub override_date: String,
    pub override_episode: i64,
    pub episodes_aired: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubEpisodeOverride {
    pub override_date: String,
    pub override_episode: i64,
    pub episodes_aired: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DubEpisodeOverride {
    pub override_date: String,
    pub override_episode: i64,
    pub episodes_aired: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Studio {
    pub name: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaType {
    pub name: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub average_score: f64,
    pub rating_count: i64,
    pub tracked_count: i64,
    pub tracked_rating: i64,
    pub color_light_mode: String,
    pub color_dark_mode: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Names {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    pub abbreviation: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Websites {
    pub official: Option<String>,
    pub mal: String,
    pub ani_list: String,
    pub kitsu: Option<String>,
    pub anime_planet: Option<String>,
    pub anidb: Option<String>,
    #[serde(default)]
    pub streams: Vec<Stream>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub platform: String,
    pub url: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relations {
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub prequels: Vec<String>,
    #[serde(default)]
    pub sequels: Vec<String>,
    #[serde(default)]
    pub side_stories: Vec<String>,
}
