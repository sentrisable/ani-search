use regex::{Regex, RegexBuilder};
use reqwest::{Client, Response, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::DEFAULT_AGENT;

const DEFAULT_ANIDB_API: &str = "https://anidb.app";
const DEFAULT_CIPHERS: &str = "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305";
const DEFAULT_TLS_CIPHERS: &str =
    "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AniDBId {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anidb_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episodes: Option<Vec<AniDbEpisode>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AniDbEpisode {
    pub filler: bool,
    pub id: u32,
    pub number: u32,
    pub number2: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct AniDbEpisodeLink {
    code: String,
    embed_url: String,
    name: String,
}

#[derive(Clone, Debug)]
struct Cached<T> {
    expires_at: Instant,
    value: T,
}

#[derive(Clone, Debug)]
pub struct AniDbClientBuilder {
    anidb_api: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for AniDbClientBuilder {
    fn default() -> Self {
        Self {
            anidb_api: DEFAULT_ANIDB_API.into(),
            user_agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(12),
        }
    }
}

impl AniDbClientBuilder {
    pub fn anidb_api(mut self, value: impl Into<String>) -> Self {
        self.anidb_api = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    pub fn build(self) -> Result<AniDbClient, Box<dyn Error>> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(AniDbClient {
            inner: Arc::new(Inner {
                http,
                anidb_api: self.anidb_api.trim_end_matches('/').into(),
                user_agent: self.user_agent,
                searches: Mutex::new(BTreeMap::new()),
                series: Mutex::new(BTreeMap::new()),
            }),
        })
    }
}

struct Inner {
    http: Client,
    anidb_api: String,
    user_agent: String,
    searches: Mutex<BTreeMap<String, Cached<Vec<Value>>>>,
    series: Mutex<BTreeMap<String, Cached<Vec<Value>>>>,
}

#[derive(Clone)]
pub struct AniDbClient {
    inner: Arc<Inner>,
}

impl AniDbClient {
    pub fn builder() -> AniDbClientBuilder {
        AniDbClientBuilder::default()
    }

    pub fn new() -> Result<AniDbClient, Box<dyn Error>> {
        Self::builder().build()
    }

    pub async fn search(&self, query: &str) -> Result<Vec<AniDBId>, Box<dyn Error>> {
        let query = query.trim().replace(" ", "+");
        let search_url = format!("{}/browse?q={}", self.inner.anidb_api, query);
        let response = self
            .inner
            .http
            .get(search_url)
            .send()
            .await?
            .text()
            .await?
            .replace("\n", " ");

        let split_response = response.split("<a href");

        let mut results = Vec::new();

        let anime_re = Regex::new(r#"anime/([a-z0-9-]+-[0-9]+).*?alt="([^"]+)""#)?;
        let id_re = Regex::new(r#"([0-9]+)"#)?;

        for line in split_response.into_iter() {
            if let Some(caps) = anime_re.captures(line) {
                let anime_id = caps.get(1).map_or("", |m| m.as_str());
                let anime_title = caps.get(2).map_or("", |m| m.as_str());

                let id_cap = match id_re.captures(anime_id) {
                    Some(id) => &id[1].to_string(),
                    None => &"".to_string(),
                };

                let episodes = self.get_episodes(id_cap).await?;

                let show_id = AniDBId {
                    anidb_id: Some(id_cap.to_string()),
                    title: Some(anime_title.to_string()),
                    episodes,
                };
                results.push(show_id);
            };
        }

        Ok(results)
    }

    pub async fn get_episodes(
        &self,
        id: &str,
    ) -> Result<Option<Vec<AniDbEpisode>>, Box<dyn Error>> {
        let search_url = format!(
            "{}/api/frontend/anime/{}/episodes",
            self.inner.anidb_api, id
        );

        let response: Value = self
            .inner
            .http
            .get(&search_url)
            .send()
            .await?
            .json()
            .await?;

        //dbg!(&response);

        let mut ep_vec = vec![];
        if let Some(episodes) = response["episodes"].as_array() {
            for episode in episodes {
                let ep_struct: AniDbEpisode = serde_json::from_value(episode.clone())?;
                ep_vec.push(ep_struct);
            }
            Ok(Some(ep_vec))
        } else {
            Ok(None)
        }
    }

    pub async fn get_episode_m3u8(
        &self,
        episode_id: u32,
    ) -> Result<BTreeMap<String, BTreeMap<String, Vec<String>>>, Box<dyn Error>> {
        let search_url = format!(
            "{}/api/frontend/episode/{}/languages",
            self.inner.anidb_api, episode_id
        );

        let response: Value = self
            .inner
            .http
            .get(&search_url)
            .send()
            .await?
            .json()
            .await?;

        dbg!(&response);

        let mut language_map: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        if let Some(languages) = response["languages"].as_array() {
            for language in languages {
                let episode: AniDbEpisodeLink = serde_json::from_value(language.clone())?;
                let m3u8_link = self.get_m3u8_address(&episode).await?;
                let link_map = self.get_video_link(m3u8_link).await?;
                language_map.insert(episode.code, link_map);
            }
        }

        Ok(language_map)
    }

    async fn get_m3u8_address(&self, episode: &AniDbEpisodeLink) -> Result<String, Box<dyn Error>> {
        let embed_response = self
            .inner
            .http
            .get(&episode.embed_url)
            .send()
            .await?
            .text()
            .await?;

        let file_re = Regex::new(r#".*file: '([^']*)'.*"#)?;
        if let Some(caps) = file_re.captures(&embed_response) {
            return Ok(caps[1].to_string());
        }

        Ok(String::new())
    }

    async fn get_video_link(
        &self,
        link: String,
    ) -> Result<BTreeMap<String, Vec<String>>, Box<dyn Error>> {
        let link_response = self
            .inner
            .http
            .get(&link)
            .send()
            .await?
            .text()
            .await?
            .replace("\n", "");
        //        dbg!(&link_response);
        let mut link_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let ext_x_re = Regex::new(r#"(#EXT-X.*?m3u8)"#)?;
        let resolution_re =
            Regex::new(r#".*?RESOLUTION=[0-9]+x([0-9]+).*?(https:\/\/[a-zA-Z0-9.\/_-]+)"#)?;
        //let mut link_vec = vec![];
        for ext in ext_x_re.captures_iter(&link_response) {
            if let Some(resolutions) = resolution_re.captures(&ext[1]) {
                //              dbg!(&resolutions);
                let key = format!("{}p", &resolutions[1]);
                link_map
                    .entry(key)
                    .or_default()
                    .push(resolutions[2].to_string());
            }
        }
        Ok(link_map)
    }
}
