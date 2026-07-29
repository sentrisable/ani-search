use std::{
    time::{Instant, Duration}, 
    error::Error, 
    sync::{Arc, Mutex},
    collections::HashMap
};
use regex::Regex;
use reqwest::{Client, Response, StatusCode, header};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};

use crate::{
    DEFAULT_AGENT,
};


const DEFAULT_ANIDB_API: &str = "https://anidb.app";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AniDBId {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anidb_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episodes: Option<u32>,
}


#[derive(Clone, Debug, PartialEq, Eq)]
struct AniDbEpisode {
    number: String,
    embed_id: Option<String>,
    sub_url: Option<String>,
    dub_url: Option<String>,
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
                searches: Mutex::new(HashMap::new()),
                series: Mutex::new(HashMap::new()),
            }),
        })
    }
}

struct Inner {
    http: Client,
    anidb_api: String,
    user_agent: String,
    searches: Mutex<HashMap<String, Cached<Vec<Value>>>>,
    series: Mutex<HashMap<String, Cached<Vec<Value>>>>,
}

#[derive(Clone)]
pub struct AniDbClient {
    inner: Arc<Inner>,
}

impl AniDbClient{
    pub fn builder() ->AniDbClientBuilder{
        AniDbClientBuilder::default()
    }

    pub fn new()->Result<AniDbClient, Box<dyn Error>>{
        Self::builder().build()
    }

    pub async fn search(&self, query: &str) -> Result<Vec<AniDBId>, Box<dyn Error>>{
        
        let query = query.trim().replace(" ", "+");
        let search_url = format!("{}/browse?q={}",self.inner.anidb_api,query);
        let response = self.inner.http
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
        
        
        for line in split_response.into_iter(){
            dbg!(&line);
            if let Some(caps) = anime_re.captures(&line){
                let anime_id = caps.get(1).map_or("", |m| m.as_str());
                let anime_title = caps.get(2).map_or("", |m| m.as_str());

                let id_cap = match id_re.captures(anime_id){
                    Some(id) => &id[1].to_string(),
                    None => &"".to_string()
                };

                let episode_count = self.get_episodes(&id_cap).await?;

                let show_id = AniDBId{
                    anidb_id: Some(id_cap.to_string()),
                    title: Some(anime_title.to_string()),
                    episodes: episode_count
                };
                results.push(show_id);    

            };


        }
        
        Ok(results)
    }

    pub async fn get_episodes(&self, mut id: &str) -> Result<Option<u32>, Box<dyn Error>>{
        let search_url = format!("{}/api/frontend/anime/{}/episodes",self.inner.anidb_api, id);

        let response: Value = self.inner.http
        .get(&search_url)
        .send()
        .await?
        .json()
        .await?;

        if let Some(episodes) = response["episodes"].as_array(){
            Ok(Some(episodes.len() as u32))
        } else{
            Ok(None)
        }
        

    }

}
