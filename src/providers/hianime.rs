use regex::{Regex, RegexBuilder};
use reqwest::{Client, Response, StatusCode, header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use base64::prelude::*;

use crate::DEFAULT_AGENT;

const DEFAULT_HIANIME_API: &str = "https://hianime.at";
const DEFAULT_CIPHERS: &str = "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305";
const DEFAULT_TLS_CIPHERS: &str =
    "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HiAnimeId {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hianime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episodes: Option<Vec<HiAnimeEpisode>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiAnimeEpisode {
    pub id: u32,
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HiAnimeEpisodeLink {
    pub quality: String,
    pub referrer: String,
    pub embed_url: String,
    pub sub: Vec<Subtitles>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Subtitles {
    pub default: bool,
    pub label: String,
    pub lang: String,
    pub src: String,
}


#[derive(Clone, Debug)]
struct Cached<T> {
    expires_at: Instant,
    value: T,
}

#[derive(Clone, Debug)]
pub struct HiAnimeClientBuilder {
    hianime_api: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for HiAnimeClientBuilder {
    fn default() -> Self {
        Self {
            hianime_api: DEFAULT_HIANIME_API.into(),
            user_agent: DEFAULT_AGENT.into(),
            timeout: Duration::from_secs(12),
        }
    }
}

impl HiAnimeClientBuilder {
    pub fn hianime_api(mut self, value: impl Into<String>) -> Self {
        self.hianime_api = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    pub fn build(self) -> Result<HiAnimeClient, Box<dyn Error>> {
        let http = Client::builder()
            .timeout(self.timeout)
            .user_agent(&self.user_agent)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(HiAnimeClient {
            inner: Arc::new(Inner {
                http,
                hianime_api: self.hianime_api.trim_end_matches('/').into(),
                user_agent: self.user_agent,
                searches: Mutex::new(BTreeMap::new()),
                series: Mutex::new(BTreeMap::new()),
            }),
        })
    }
}

struct Inner {
    http: Client,
    hianime_api: String,
    user_agent: String,
    searches: Mutex<BTreeMap<String, Cached<Vec<Value>>>>,
    series: Mutex<BTreeMap<String, Cached<Vec<Value>>>>,
}

#[derive(Clone)]
pub struct HiAnimeClient {
    inner: Arc<Inner>,
}

impl HiAnimeClient {
    pub fn builder() -> HiAnimeClientBuilder {
        HiAnimeClientBuilder::default()
    }

    pub fn new() -> Result<HiAnimeClient, Box<dyn Error>> {
        Self::builder().build()
    }

    pub async fn search(&self, query: &str) -> Result<Vec<HiAnimeId>, Box<dyn Error>> {
        let query = query.trim().replace(" ", "+");
        
        let search_url = format!("{}/search?keyword={}", self.inner.hianime_api, query);
        let response = self
            .inner
            .http
            .get(search_url)
            .send()
            .await?
            .text()
            .await?
            .replace("\n", " ");
        let mut results = Vec::new();
        let content_before_sidebar = match response.as_str().find("id=\"main-sidebar\""){
            Some(index)  => &response[..index],
            None => &response
        };
        let single_line = content_before_sidebar.replace("\n", " "); //Flatten response
        let structed_lines = single_line.replace("<div class=\"film-detail\">", "\n<div class=\"film-detail\">");//Structure text at film detail divs
        let film_regex = Regex::new(
        r#"(?x)
        <h3\s+class="film-name">
        \s*
        <a\s+href="[^"]*/([^"/]*)"
        \s+
        title="([^"]*)"
    "#,
    )?;

        for line in structed_lines.lines() {
            if let Some(caps) = film_regex.captures(line) {
                
                let anime_id = &caps[1];
                let anime_title = &caps[2];
                

                let episodes = self.get_episodes(&anime_id).await?;
                let show_id = HiAnimeId {
                    hianime_id: Some(anime_id.to_string()),
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
    ) -> Result<Option<Vec<HiAnimeEpisode>>, Box<dyn Error>> {
        dbg!(&id);
        let search_id = id.split('-').last().unwrap_or(id);
        let search_url = format!(
            "{}/api/theme/episode/list/{}",
            self.inner.hianime_api, search_id
        );

        
        let response = self
            .inner
            .http
            .get(&search_url)
            .send()
            .await?
            .text()
            .await?;

        let formatted_response = response.replace("\\", "").replace("ep-item", "\nep-item");
        let episode_parse = Regex::new(r#".*data-number="([^"]*)".*data-id="([0-9]+)".*"#)?;
        let mut ep_vec = vec![];
        let caps = episode_parse.captures_iter(&formatted_response);
        for cap in caps{
                if let Ok(ep_id) = cap[2].parse::<u32>() 
                    && let Ok(ep_number) = cap[1].parse::<u32>(){
                    let episode = HiAnimeEpisode{
                        id: ep_id,
                        number: ep_number
                    };
                    ep_vec.push(episode);    
                    }
                
            }
        Ok(Some(ep_vec))

        
        
    }

    pub async fn get_episode_m3u8(
        &self,
        mode:&str,
        episode_id: u32,
    ) -> Result<Vec<HiAnimeEpisodeLink>, Box<dyn Error>> {
        let search_url = format!(
            "{}/api/theme/episode/servers?episodeId={}",
            self.inner.hianime_api, episode_id
        );

        
        let response: String = self
            .inner
            .http
            .get(&search_url)
            .send()
            .await?
            .text()
            .await?;

        
        let formatted_response = response.replace("\\", "").replace("server-item", "\nserver-item");
        std::fs::write("response.txt", &formatted_response);
        let zoko_pattern = format!(".*data-type=\"{mode}\".*data-server-name=\"ZokoAnime\".*data-hash=\"([^\"]*)\".*");
        let zoko_re = Regex::new(&zoko_pattern)?;
        let mut link_map: Vec<HiAnimeEpisodeLink> = vec![];
        
        if let Some(caps) = zoko_re.captures(&formatted_response){
            
            let hash = &caps[1];

            let decoded_bytes = BASE64_STANDARD.decode(hash)?;
            let decoded_hash = String::from_utf8(decoded_bytes)?;


            let refr_re= Regex::new(r#"^(https?://[^/]*).*"#)?;
            let refr = match refr_re.captures(&decoded_hash){
                Some(c) => &c[1].to_string(),
                None => ""
            };

            let mal_re = Regex::new(r#".*/mal/([0-9]+)/.*"#)?;
            let mal = match mal_re.captures(&decoded_hash){
                Some(c) => &c[1].to_string(),
                None => ""
            };

            let blob = self.get_blob(&decoded_hash).await?;
            let blob_json:Value = serde_json::from_str(&blob)?;
            dbg!(&blob_json);
            let m3u8_master = match blob_json["src"].as_str(){
                Some(x) => x,
                None => ""
            };
            let subtitles = match blob_json["subtitles"].as_array(){
                Some(a) => a,
                None => &vec![]
            };
            let mut sub_list = vec![];
            for sub in subtitles{
                let y = serde_json::from_value::<Subtitles>(sub.clone())?;
                sub_list.push(y);
            }
            
            link_map = self.get_video_link(&m3u8_master, &refr, sub_list).await?;
        }
        
        Ok(link_map)
    }

    async fn get_blob(
        &self,
        hash: &str
    ) -> Result<String, Box<dyn Error>>{
        let mut blob_bytes = vec![];
        let mut blob = String::new();

        let mut key = vec![111, 116, 97, 107, 117, 45, 101, 109, 98, 101, 100, 45, 118, 49];
        
        let response: String = self
            .inner
            .http
            .get(hash)
            .send()
            .await?
            .text()
            .await?;
        let blob_re = Regex::new(r#".*window.__P="([^"]*)".*"#)?;
        if let Some(blob_caps) = blob_re.captures(&response){
            let encoded = &blob_caps[1];
            let decoded_bytes = BASE64_STANDARD.decode(encoded)?;
            for byte in decoded_bytes{
                let char = byte ^ key[0];
                blob_bytes.push(char);

                let first = key.remove(0);
                key.push(first);
            }
            blob = String::from_utf8(blob_bytes)?;
            
            }
        
        Ok(blob)

    }


    async fn get_m3u8_address(&self,

        episode: &HiAnimeEpisodeLink
    ) -> Result<String, Box<dyn Error>> {
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
        link: &str,
        referrer: &str,
        subtitles: Vec<Subtitles>
    ) -> Result<Vec<HiAnimeEpisodeLink>, Box<dyn Error>> {

        //dbg!(link);
        let link_response = self
            .inner
            .http
            .get(link)
            .header(reqwest::header::REFERER, referrer)
            .send()
            .await?
            .text()
            .await?;
        dbg!(&link_response);
        let mut link_map: Vec<HiAnimeEpisodeLink> = vec![];
        let lines:Vec<&str> = link_response.lines().map(|s| s.trim()).collect();
        for line in lines{
            dbg!(line);

            if !line.starts_with("#"){
                let split:Vec<&str> = line.split("/").collect();
                let resolution = format!("{}p", split[0]);
                let link_stub = link.replace("master.m3u8", "");
                let link = format!("{link_stub}{line}");
                let episodes = HiAnimeEpisodeLink{
                    quality: resolution,
                    referrer: referrer.to_string(),
                    embed_url: link,
                    sub: subtitles.clone()
                };
                link_map.push(episodes);
            }


            
        }
        Ok(link_map)
    }
}
