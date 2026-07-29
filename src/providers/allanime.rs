use std::{
    collections::{BTreeSet, HashMap, HashSet}, hash::Hash, sync::{mpsc::{Receiver, Sender},{Arc, Mutex}}, time::{Duration, Instant}
};
use curl::easy::{Easy2, Handler, List, ReadError, WriteError};
use regex::Regex;
use reqwest::{Client, Response, StatusCode, header};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use base64::prelude::BASE64_STANDARD;

use crate::{
    DEFAULT_AGENT, Episode, Result, SearchResult, Config, Translation,
    shows::Edge,

};


const DEFAULT_AA_REFR: &str = "https://mkissa.to";
const DEFAULT_AA_API: &str = "https://api.mkissa.net";

#[derive(Clone, Debug)]
struct Cached<T>{
    expires_at: Instant,
    value: T
}

#[derive(Clone, Debug)]
pub struct AllAnimeClientBuilder{
    all_anime_api: String,
    all_anime_refr: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for AllAnimeClientBuilder{
    fn default() -> Self {
        Self { 
            all_anime_api: DEFAULT_AA_API.into(), 
            all_anime_refr: DEFAULT_AA_REFR.into(),
            user_agent: DEFAULT_AGENT.into(), 
            timeout: Duration::from_secs(12) 
        }
    }
}


impl AllAnimeClientBuilder{
    pub fn all_anime_api(mut self, value: impl Into<String>) ->Self{
        self.all_anime_api = value.into();
        self
    }
    pub fn all_anime_refr(mut self, value: impl Into<String>) ->Self{
        self.all_anime_refr = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self{
        self.timeout = value;
        self
    }

    fn build(self) -> Result<AllAnimeClient>{
        let http = Client::builder()
        .timeout(self.timeout)
        .user_agent(&self.user_agent)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;
    Ok(AllAnimeClient {
        inner: Arc::new(Inner{
            http,
            all_anime_api: self.all_anime_api.trim_end_matches('/').into(),
            all_anime_refr: self.all_anime_refr.trim_end_matches('/').into(),
            user_agent: self.user_agent,
            searches: Mutex::new(HashMap::new()),
            series: Mutex::new(HashMap::new())
        })
    })
    }
}

struct Inner{
    http: Client,
    all_anime_api: String,
    all_anime_refr: String,
    user_agent: String,
    searches: Mutex<HashMap<String, Cached<Vec<SearchResult>>>>,
    series: Mutex<HashMap<String, Cached<Vec<Episode>>>>,
}

#[derive(Clone)]
pub struct AllAnimeClient{
    inner: Arc<Inner>
}

impl AllAnimeClient{
    pub fn builder() -> AllAnimeClientBuilder{
        AllAnimeClientBuilder::default()
    }

    pub fn new() -> Result<Self>{
        Self::builder().build()
    }

    pub async fn search(&self, query: &str, translation: &Translation, config: &Config){
        self.get_shows(query, translation, config).await;
    }

pub async fn get_shows(
    &self,
    show: &str,
    translation_type: &Translation,
    config: &Config,
) -> Result<Vec<Edge>> {
    let search_gql = "query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";

    let response = self.inner.http
        .post(&self.inner.all_anime_api)
        .header(header::REFERER, self.inner.all_anime_refr.clone())
        .json(&json!({
        "variables":{
            "search":{
                "allowAdult": &config.app_settings.allow_adult,
                "allowUnknown": false,
                "query": show
            },
            "limit": 40,
            "page": 1,
            "translationType" : match translation_type {
                Translation::Sub => "sub",
                Translation::Dub => "dub",
                },
            "countryOrigin": "ALL"
        },
        "query": search_gql
    })
    ).send()
    .await?;

    dbg!(&response);

    let json = check_json(response, "AllAnime").await?;

    println!("{:?}",&json);
    Ok(Vec::new())
}



}

async fn check_json(response: Response, provider: &str) -> Result<Value>{
    let status = response.status();
    
    if status == StatusCode::TOO_MANY_REQUESTS{
        let retry_after_seconds = response.headers().get(header::RETRY_AFTER).and_then(|value| value.to_str().ok()).and_then(|value| value.parse().ok()).unwrap_or(120);
        return Err("Rate Limited".into());
    }
    if !status.is_success(){
        return Err(format!("{status}").into());
    }
    response.json().await.map_err(Into::into)
}


// fn merge_to_key(part_a_hex: &str, part_b_hex: &str) -> Result<String> {
//     let part_a = hex::decode(&part_a_hex)?;
//     let part_b = BASE64_STANDARD.decode(part_b_hex)?;

//     let key: String = part_a
//         .iter()
//         .zip(part_b.iter())
//         .map(|(m_byte, p_byte)| format!("{:02x}", m_byte ^ p_byte))
//         .collect();
//     dbg!(&key);
//     Ok(key)
// }

// fn parse_chunks(
//     imm_url: &str,
//     unique_chunks: BTreeSet<String>,
// ) -> Result<String> {
//     let mut chunk = String::new();

//     let mask_re = regex::Regex::new(r"[a-f0-9]{64}")?;

//     for c in unique_chunks {
//         if c.is_empty() {
//             continue;
//         }

//         let chunk_url = format!("{imm_url}{c}");

//         let payload = String::new();
//         let handler = PostHandler {
//             upload_data: payload.clone(),
//             response_data: Vec::new(),
//         };
//         let mut handle = Easy2::new(handler);
//         //handle.post(true)?;
//         //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//         handle.url(&chunk_url)?;

//         handle.useragent(
//             "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//         )?;

//         #[cfg(debug_assertions)]
//         handle.verbose(false)?;

//         handle.perform()?;

//         let contents = handle.get_ref();
//         let response = &std::str::from_utf8(&contents.response_data)?;
//         if !response.contains("__aaCrypto") {
//             continue;
//         }

//         let masks: Vec<&str> = mask_re.find_iter(*response).map(|m| m.as_str()).collect();

//         if masks.len() == 1 {
//             chunk = masks[0].to_string();

//             return Ok(chunk);
//         }
//     }
//     Ok(String::new())
// }

// fn curl_cdn_immutable(entry: &str) -> Result<String> {
//     let immutable_url = "https://cdn.allanime.day/all/mk/_app/immutable/";

//     let payload = String::new();
//     let handler = PostHandler {
//         upload_data: payload.clone(),
//         response_data: Vec::new(),
//     };
//     let mut handle = Easy2::new(handler);
//     //handle.post(true)?;
//     //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//     handle.url(format!("{immutable_url}{entry}").as_str())?;

//     handle.useragent(
//         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//     )?;

//     #[cfg(debug_assertions)]
//     handle.verbose(true)?;

//     handle.perform()?;

//     let contents = handle.get_ref();
//     let response = &std::str::from_utf8(&contents.response_data)?;
//     //dbg!(response);
//     let chunk_re = regex::Regex::new(r#"["']\.\./(chunks/[a-zA-Z0-9_.-]+\.js)["']"#)?;
//     let mut unique_chunks = BTreeSet::new();
//     for cap in chunk_re.captures_iter(*response) {
//         if let Some(chunk) = cap.get(1) {
//             unique_chunks.insert(chunk.as_str().to_string());
//         }
//     }
//     //dbg!(&unique_chunks);
//     let chunk = parse_chunks(immutable_url, unique_chunks)?;
//     if chunk.is_empty() {
//         return Err("Unable to parse chunks".into());
//     }
//     Ok(chunk)
// }

// pub fn get_allanime_key() -> Result<AllAnimeKey> {
//     let payload = String::new();
//     let handler = PostHandler {
//         upload_data: payload.clone(),
//         response_data: Vec::new(),
//     };
//     let mut handle = Easy2::new(handler);

//     //handle.post(true)?;
//     //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//     handle.url("https://mkissa.to")?;

//     handle.useragent(
//         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//     )?;

//     #[cfg(debug_assertions)]
//     handle.verbose(true)?;

//     handle.perform()?;
//     let mut key_struct = AllAnimeKey::default();
//     let contents = handle.get_ref();
//     let response = &std::str::from_utf8(&contents.response_data)?;
//     //dbg!(response);

//     let aacrypto_re = regex::Regex::new(r#"window\.__aaCrypto\s*=\s*(\{[^}]*\})"#)?;
//     let appjs_re = regex::Regex::new(r#"_app/immutable/(entry/app\.[^"']+\.js)"#)?;

//     if let Some(caps) = aacrypto_re.captures(*response) {
//         let json: Aacrypto = serde_json::from_str(&caps[1]).unwrap();

//         if let Some(js_caps) = appjs_re.captures(*response) {
//             let entry = &js_caps[1];
//             let part_a_hex = curl_cdn_immutable(&entry)?;
//             key_struct.epoch = json.epoch;
//             key_struct.part_a_hex = part_a_hex;
//             key_struct.part_b_hex = json.part_b;
//         }
//     }
//     dbg!(&key_struct);
//     Ok(key_struct)
// }

// pub fn write_to_log(
//     message: &str,
//     message_type: MessageType,
// ) -> Result<()> {
//     #[cfg(target_os = "linux")]
//     let path = "./logs/errors.log";

//     #[cfg(target_os = "windows")]
//     let path = ".\\logs\\errors.log";

//     let mut log = std::fs::OpenOptions::new()
//         .append(true)
//         .create(true)
//         .open(path)?;
//     match message_type {
//         MessageType::Error => {
//             log.write_all(format!("ERROR: {} - {}", message, chrono::Local::now()).as_bytes())?;
//         }
//         MessageType::Informational => {
//             log.write_all(format!("INFO: {} - {}", message, chrono::Local::now()).as_bytes())?;
//         }
//     }
//     Ok(())
// }

// pub fn create_file(path: &str) -> Result<()> {
//     if !std::path::Path::new(path).exists() {
//         write_to_log(
//             format!("Creating {path}...").as_str(),
//             MessageType::Informational,
//         );
//         fs::File::create_new(path)?;
//         write_to_log(
//             format!("{path} created.").as_str(),
//             MessageType::Informational,
//         );
//     } else {
//         let message = format!("{path} already exists");
//         write_to_log(&message, MessageType::Error);
//     }
//     Ok(())
// }

// fn get_filemoon_link(source_id: &str) -> Result<()> {
//     let payload = String::new();
//     let handler = PostHandler {
//         upload_data: payload.clone(),
//         response_data: Vec::new(),
//     };
//     let mut handle = Easy2::new(handler);
//     handle.referer("https://mkissa.to")?;
//     dbg!(&source_id);
//     let url = format!("https://allanime.day");
//     handle.url(&url)?;

//     handle.useragent(
//         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//     )?;

//     #[cfg(debug_assertions)]
//     handle.verbose(true)?;

//     handle.perform()?;

//     let contents = handle.get_ref();
//     let response = &std::str::from_utf8(&contents.response_data)?;
//     println!("{response}");
//     Ok(())
// }

// fn extract_link(
//     episode_link: &str,
//     response: &str,
// ) -> Result<Vec<(i32, String)>> {
//     use regex::Regex;
//     match episode_link {
//         x if x.contains("repackager.wixmp.com") => {
//             let extracted_link = x
//                 .replace("repackager.wixmp.com/", "")
//                 .split(".urlset")
//                 .next()
//                 .unwrap_or("")
//                 .to_string();
//             let re = Regex::new(r".*/,[^/],/mp4.*").unwrap();

//             let mut output: Vec<(i32, String)> = Vec::new();

//             if let Some(caps) = re.captures(x) {
//                 let csv_group = &caps[1];
//                 for j in csv_group.split(',') {
//                     if j.is_empty() {
//                         continue;
//                     }
//                     let formatted_line = format!("{} > {}", j, extracted_link).replace(",[^/]*", j);
//                     let numeric_key: i32 = j
//                         .chars()
//                         .filter(|c| c.is_ascii_digit())
//                         .collect::<String>()
//                         .parse()?;
//                     output.push((numeric_key, formatted_line));
//                 }
//             }
//             output.sort_by(|a, b| b.0.cmp(&a.0));
//             Ok(output)
//         }
//         x if x.contains("master.m3u8") => {
//             let re = Regex::new(r#"Referer":"([^"]*)""#)?;
//             let m3u8_refr = re
//                 .captures(response)
//                 .map(|caps| caps[1].to_string())
//                 .unwrap_or_default();
//             println!("{x}");
//             let first_line = x.lines().next().unwrap_or("");
//             let extract_link = first_line.split('>').nth(1).unwrap_or("");
//             let relative_link = match extract_link.rfind('/') {
//                 Some(idx) => &x[..idx + 1],
//                 None => "",
//             };

//             let payload = String::new();
//             let handler = PostHandler {
//                 upload_data: payload.clone(),
//                 response_data: Vec::new(),
//             };
//             let mut handle = Easy2::new(handler);
//             handle.referer(&m3u8_refr)?;

//             let mut headers = List::new();
//             headers.append("Content-Type: application/json")?;
//             handle.http_headers(headers)?;

//             //handle.post(true)?;
//             //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//             handle.url(&extract_link)?;

//             handle.useragent(
//                 "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//             )?;

//             #[cfg(debug_assertions)]
//             handle.verbose(true)?;

//             handle.perform()?;

//             let contents = handle.get_ref();
//             let response = &std::str::from_utf8(&contents.response_data)?;
//             println!("{response}");

//             Ok(Vec::new())
//         }
//         x => {
//             let link_vec = vec![(0, x.to_string())];
//             Ok(link_vec)
//         }
//     }
// }

// fn get_episode_link(source_id: &str) -> Result<Vec<(i32, String)>> {
//     //todo!("Finish grabbing links for episodes");

//     let ref_url = "https://mkissa.to";
//     let api_base = "allanime.day";
//     match source_id.to_lowercase() {
//         x if x.contains("mp4upload") => {
//             println!("{x}");
//             let payload = String::new();
//             let handler = PostHandler {
//                 upload_data: payload.clone(),
//                 response_data: Vec::new(),
//             };
//             let mut handle = Easy2::new(handler);
//             handle.referer(ref_url)?;
//             handle.timeout(Duration::from_secs(10))?;
//             handle.follow_location(true)?;
//             handle.ssl_verify_host(false)?;
//             handle.ssl_verify_peer(false)?;

//             let mut headers = List::new();
//             headers.append("Content-Type: application/json")?;
//             handle.http_headers(headers)?;

//             //handle.post(true)?;
//             //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//             handle.url(&x)?;

//             handle.useragent(
//                 "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//             )?;

//             #[cfg(debug_assertions)]
//             handle.verbose(true)?;

//             handle.perform()?;

//             let contents = handle.get_ref();
//             let response = &std::str::from_utf8(&contents.response_data)?;
//             //println!("{response}");
//             let source_re = regex::Regex::new(r#".*src: "([^"]*)"\s*"#)?;
//             if let Some(caps) = source_re.captures(*response) {
//                 let links = extract_link(&caps[1], "")?;
//                 println!("{links:?}");
//                 Ok(links)
//             } else {
//                 Err("Unable to capture source link".into())
//             }

//             //Ok(String::new())
//         }
//         x if x.contains("tools.fast4speed.rsvp") => {
//             let episode_link = vec![(0, x)];
//             Ok(episode_link)
//         }
//         x => {
//             let payload = String::new();
//             let handler = PostHandler {
//                 upload_data: payload.clone(),
//                 response_data: Vec::new(),
//             };
//             let mut handle = Easy2::new(handler);
//             handle.referer(ref_url)?;

//             let mut headers = List::new();
//             headers.append("Content-Type: application/json")?;
//             handle.http_headers(headers)?;

//             //handle.post(true)?;
//             //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//             handle.url(&format!("https://{api_base}{x}"))?;

//             handle.useragent(
//                 "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//             )?;

//             #[cfg(debug_assertions)]
//             handle.verbose(true)?;

//             handle.perform()?;

//             let contents = handle.get_ref();
//             let response = &std::str::from_utf8(&contents.response_data)?;
//             let root: episode_links::Root = serde_json::from_str(response)?;

//             let links = extract_link(&root.links[0].link, response)?;

//             //Ok(response.clone())
//             Ok(links)
//         }
//     }
// }

// fn source_init(source_name: &str, source: &SourceUrl) -> String {
//     if let Some(source_url) = &source.source_url {
//         if !source_url.starts_with("--") {
//             return source_url.clone();
//         }

//         let payload = &source_url[2..];
//         let mut decoded = String::with_capacity(payload.len() / 2);

//         let mut chars = payload.chars();
//         while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
//             let pair: String = format!("{c1}{c2}");

//             let mapped_char = match pair.as_str() {
//                 "79" => 'A',
//                 "7a" => 'B',
//                 "7b" => 'C',
//                 "7c" => 'D',
//                 "7d" => 'E',
//                 "7e" => 'F',
//                 "7f" => 'G',
//                 "70" => 'H',
//                 "71" => 'I',
//                 "72" => 'J',
//                 "73" => 'K',
//                 "74" => 'L',
//                 "75" => 'M',
//                 "76" => 'N',
//                 "77" => 'O',
//                 "68" => 'P',
//                 "69" => 'Q',
//                 "6a" => 'R',
//                 "6b" => 'S',
//                 "6c" => 'T',
//                 "6d" => 'U',
//                 "6e" => 'V',
//                 "6f" => 'W',
//                 "60" => 'X',
//                 "61" => 'Y',
//                 "62" => 'Z',
//                 "59" => 'a',
//                 "5a" => 'b',
//                 "5b" => 'c',
//                 "5c" => 'd',
//                 "5d" => 'e',
//                 "5e" => 'f',
//                 "5f" => 'g',
//                 "50" => 'h',
//                 "51" => 'i',
//                 "52" => 'j',
//                 "53" => 'k',
//                 "54" => 'l',
//                 "55" => 'm',
//                 "56" => 'n',
//                 "57" => 'o',
//                 "48" => 'p',
//                 "49" => 'q',
//                 "4a" => 'r',
//                 "4b" => 's',
//                 "4c" => 't',
//                 "4d" => 'u',
//                 "4e" => 'v',
//                 "4f" => 'w',
//                 "40" => 'x',
//                 "41" => 'y',
//                 "42" => 'z',
//                 "08" => '0',
//                 "09" => '1',
//                 "0a" => '2',
//                 "0b" => '3',
//                 "0c" => '4',
//                 "0d" => '5',
//                 "0e" => '6',
//                 "0f" => '7',
//                 "00" => '8',
//                 "01" => '9',
//                 "15" => '-',
//                 "16" => '.',
//                 "67" => '_',
//                 "46" => '~',
//                 "02" => ':',
//                 "17" => '/',
//                 "07" => '?',
//                 "1b" => '#',
//                 "63" => '[',
//                 "65" => ']',
//                 "78" => '@',
//                 "19" => '!',
//                 "1c" => '$',
//                 "1e" => '&',
//                 "10" => '(',
//                 "11" => ')',
//                 "12" => '*',
//                 "13" => '+',
//                 "14" => ',',
//                 "03" => ';',
//                 "05" => '=',
//                 "1d" => '%',
//                 _ => ' ',
//             };
//             decoded.push(mapped_char);
//         }

//         decoded.replace("/clock", "/clock.json")
//     } else {
//         return String::from("Unable to Parse Url");
//     }
// }

// pub fn generate_link(source: &SourceUrl) -> Result<Vec<(i32, String)>> {
//     if let Some(source_name) = &source.source_name {
//         match source_name.as_str() {
//             "Mp4" => {
//                 let source_id = source_init("mp4upload", source);
//                 let episode_link = get_episode_link(&source_id)?;

//                 dbg!(&episode_link);
//                 Ok(episode_link)
//             }
//             // "Fm-Hls" => {

//             //     let source_id = source_init("Filemoon", source);
//             //     dbg!(source_id);
//             //     //get_filemoon_link(source_id.as_str());
//             //     Ok(Vec::new())
//             //     },
//             "Yt-mp4" => {
//                 let source_id = source_init("youtube", source);
//                 let episode_link = get_episode_link(&source_id)?;

//                 dbg!(&episode_link);
//                 Ok(episode_link)
//             }
//             "S-mp4" => {
//                 let source_id = source_init("sharepoint", source);
//                 let episode_link = get_episode_link(&source_id)?;

//                 dbg!(&episode_link);
//                 Ok(episode_link)
//             }
//             _ => Ok(Vec::new()),
//         }
//     } else {
//         Err("Unable to get Source Name".into())
//     }
// }

// fn encrypt_key(key: &str) -> Vec<u8> {
//     use sha2::{Digest, Sha256};
//     let mut hasher = Sha256::new();
//     hasher.update(key);
//     let hash_result = hasher.finalize();

//     hash_result.to_vec()
// }

// fn decrypt_response(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<String> {

//     use openssl::symm::{decrypt, Cipher};
//     let
//     let decoded_iv = hex::decode(iv)?;

//     let decrypted_bytes = decrypt(Cipher::aes_256_ctr(), key, Some(&decoded_iv), ciphertext)?;

//     let decrypted_text = String::from_utf8(decrypted_bytes)?;

//     Ok(decrypted_text)
// }

// fn process_response(
//     url_data: &episodes::URLData,
//     key: &str,
// ) -> Result<episode_source::Root>> {
//     use aes::cipher::{KeyIvInit, StreamCipher};
//     type Aes256Ctr64Be = ctr::Ctr64BE<aes::Aes256>;

//     let to_be_parsed = &url_data.tobeparsed;
//     //println!("{to_be_parsed}");
//     let bytes = base64::prelude::BASE64_STANDARD.decode(to_be_parsed)?;

//     let buffer = &bytes[1..13];
//     let mut ctr_block = [0u8; 16];
//     ctr_block[..12].copy_from_slice(buffer);
//     ctr_block[12..16].copy_from_slice(&[0x00, 0x00, 0x00, 0x02]);

//     let ct_len = bytes.len() - 16;

//     let mut encrypted_buffer = bytes[13..(ct_len)].to_vec();

//     let decoded_key = hex::decode(key)?;
//     let mut decryptor = Aes256Ctr64Be::new_from_slices(&decoded_key, &ctr_block)?;
//     decryptor.apply_keystream(&mut encrypted_buffer);

//     let response = String::from_utf8(encrypted_buffer.to_vec())?;

//     dbg!(&response);
//     let response_json: episode_source::Root = serde_json::from_str(&response)?;
//     Ok(response_json)
// }

// fn get_aa_req(
//     aa_key: &str,
//     epoch: i32,
//     query_hash: &str,
// ) -> Result<String>> {
//     use aes_gcm::{
//         Aes256Gcm, Key, Nonce,
//         aead::{Aead, KeyInit},
//     };

//     use chrono::Utc;
//     use hybrid_array::{Array, sizes};
//     let ts = (Utc::now().timestamp() / 300) * 300 * 1000;
    

//     let decoded_key = hex::decode(aa_key)?;

//     let payload_iv = format!("{epoch}{query_hash}:{ts}");
//     let payload = json!({
//         "v": 1,
//         "ts": ts,
//         "epoch": epoch,
//         "qh": query_hash
//     })
//     .to_string();

//     let encrypted_iv = encrypt_key(&payload_iv);
//     let encrypted_bytes = &encrypted_iv[..12];
//     dbg!(&aa_key.as_bytes().len());
//     // let key: &Array<u8, sizes::U32> = &Key::<Aes256Gcm>::try_from(&decoded_key as &[u8])?;
//     let cipher = Aes256Gcm::new_from_slice(&decoded_key)?;
//     let nonce: &Array<u8, sizes::U12> = &Nonce::try_from(encrypted_bytes)?;

//     let cipher_text = cipher.encrypt(nonce, payload.as_bytes())?;

//     let mut buffer = Vec::with_capacity(1 + 12 + cipher_text.len());
//     buffer.push(0x01);
//     buffer.extend_from_slice(encrypted_bytes);
//     buffer.extend_from_slice(&cipher_text);

//     let b64_string = base64::prelude::BASE64_STANDARD.encode(&buffer);

//     Ok(b64_string)
// }

// fn post_curl(payload: String) -> Result<String>> {
//     let ref_url = "https://mkissa.to";
//     let api_base = "allanime.day";
//     let api_url = format!("https://api.{}", api_base);

//     let handler = PostHandler {
//         upload_data: payload.clone(),
//         response_data: Vec::new(),
//     };
//     let mut handle = Easy2::new(handler);
//     handle.referer(ref_url)?;

//     let mut headers = List::new();
//     headers.append("Content-Type: application/json")?;
//     handle.http_headers(headers)?;

//     handle.post(true)?;
//     handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

//     handle.url(&format!("{api_url}/api/"))?;

//     handle.useragent(
//         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//     )?;

//     #[cfg(debug_assertions)]
//     handle.verbose(true)?;

//     handle.perform()?;

//     let contents = handle.get_ref();
//     let response = &std::str::from_utf8(&contents.response_data)?.replace("_", "");
//     Ok(response.clone())
// }


// pub fn get_episode_list(
//     show_id: &String,
// ) -> Result<episodes::AvailableEpisodesDetail> {
//     let episode_list_gql =
//         "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";

//     let payload =
//         format!("{{\"variables\":{{\"showId\":\"{show_id}\"}},\"query\":\"{episode_list_gql}\"}}");

//     let response = post_curl(payload)?;
//     // let json:Value = serde_json::from_str(&response)?;
//     // println!("{:?}", json);

//     let data: episodes::Root = serde_json::from_str(&response)?;
//     dbg!(&data);

//     Ok(data.data.show.available_episodes_detail)
// }

// pub fn get_episode_url(
//     show_id: &String,
//     translation_type: &String,
//     episode_num: &String,
// ) -> Result<Vec<(String, Vec<(i32, String)>)>> {
//     let episode_embed_gql = "query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
//     let key_struct = get_allanime_key()?;

//     let key = merge_to_key(&key_struct.part_a_hex, &key_struct.part_b_hex)?;

//     let query_hash = "f4662f4b7510b26795dd53ef824a0bf1740fbbc5d1273fab18222ac831bca8d0";

//     let aa_req = get_aa_req(&key, key_struct.epoch, query_hash)?;
//     dbg!(&aa_req);
//     let query_vars = json!({
//         "showId": show_id,
//         "translationType": translation_type,
//         "episodeString": episode_num
//     })
//     .to_string();
//     //let query_vars=format!("{{\"showId\":\"{show_id}\",\"translationType\":\"{translation_type}\",\"episodeString\":\"{episode_num}\"}}");
//     let query_ext = json!({
//         "persistedQuery" : {
//             "version": 1,
//             "sha256Hash": query_hash
//         },
//         "aaReq": aa_req
//     })
//     .to_string();
//     //let query_ext=format!("{{\"persistedQuery\":{{\"version\":1,\"sha256Hash\":\"{query_hash}\"}}, \"aaReq\":\"{aa_req}\" }}");

//     let payload = format!(
//         "variables={}&extensions={}",
//         encode(&query_vars),
//         encode(&query_ext)
//     );
//     dbg!(&payload);

//     let ref_url = "https://mkissa.to";
//     let api_base = "allanime.day";
//     let api_url = format!("https://api.mkissa.net");

//     let handler = PostHandler {
//         upload_data: payload.clone(),
//         response_data: Vec::new(),
//     };

//     let mut handle = Easy2::new(handler);

//     handle.referer(ref_url)?;

//     let mut headers = List::new();
//     headers.append("Content-Type: application/json")?;
//     handle.http_headers(headers)?;

//     handle.get(true)?;
//     //handle.post_field_size(payload.as_bytes().len() as u64)?;

//     handle.url(&format!("{api_url}/api?{payload}"))?;

//     handle.useragent(
//         "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
//     )?;

//     handle.verbose(false)?;

//     handle.perform()?;

//     let contents = handle.get_ref();
//     let response = std::str::from_utf8(&contents.response_data)?;
//     dbg!(&response);

//     let root: episodes::URLRoot = serde_json::from_str(response)?;
//     // if root.data.tobeparsed.is_empty(){
//     //     let payload = format!("{{\"variables\":{query_vars},\"query\":\"{episode_embed_gql}\" }}");
//     //     let response = post_curl(payload)?;
//     //     let
//     // } else {
//     let response = process_response(&root.data, &key)?;
//     if let Some(episode) = response.episode {
//         if let Some(source_url) = episode.source_urls {
//             let mut urls: Vec<(String, Vec<(i32, String)>)> = Vec::new();
//             for url in source_url {
//                 let source_name = match &url.source_name {
//                     Some(source) => source,
//                     None => "No Source available",
//                 };
//                 dbg!(&source_name);
//                 let episode_link = generate_link(&url)?;
//                 urls.push((source_name.to_string(), episode_link));
//             }
//             Ok(urls)
//         } else {
//             Err("Unable to get source Urls".into())
//         }
//     } else {
//         Err("Unable to get Episode Information".into())
//     }

//     //}
// }