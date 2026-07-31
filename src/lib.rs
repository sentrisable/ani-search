use core::fmt;
use std::{
    collections::BTreeSet,
    error::Error,
    fmt::write,
    fs,
    io::{BufWriter, Read, Write, stdout},
    path::*,
    process::Command,
    string,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use base64::Engine;

use chrono::format::parse;
use curl::easy::{Easy2, Handler, List, ReadError, WriteError};
use egui::Key::V;
use hybrid_array::{ArraySize, typenum};
use openssl_sys::EVP_MAC_CTX_new;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::repeat;
use urlencoding::encode;

mod apis;
mod providers;
mod structs;

pub use apis::*;
pub use providers::*;
pub use structs::*;

const DEFAULT_AA_API: &str = "https://api.mkissa.net";
const DEFAULT_REF_URL: &str = "https://mkissa.to";
const DEFAULT_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum VideoPlayer {
    #[default]
    MPV,
    VLC,
}

impl fmt::Display for VideoPlayer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VideoPlayer::MPV => write!(f, "MPV"),
            VideoPlayer::VLC => write!(f, "VLC"),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub allow_adult: bool,
    pub video_player: VideoPlayer,
    pub download_dir: String,
    pub initialized: bool,
}

pub struct SettingsIter<'a> {
    setting: &'a AppSettings,
    state: usize,
}

impl AppSettings {
    pub fn iter(&self) -> SettingsIter {
        SettingsIter {
            setting: self,
            state: 0,
        }
    }
}

impl<'a> Iterator for SettingsIter<'a> {
    type Item = (&'static str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.state {
            0 => Some(("Allow Adult", self.setting.allow_adult)),
            _ => None,
        };
        self.state += 1;
        result
    }
}

#[derive(Default, Debug)]
pub struct AllAnimeKey {
    epoch: i32,
    mask: String,
    lane: String,
    build_id: String,
    part_b: String,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub app_settings: AppSettings,
    pub anilist: AnilistVariables,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Translation {
    Sub,
    Dub,
}

pub struct PostHandler {
    upload_data: String,
    response_data: Vec<u8>,
}

impl Handler for PostHandler {
    fn read(&mut self, data: &mut [u8]) -> std::result::Result<usize, ReadError> {
        let size = self
            .upload_data
            .as_bytes()
            .read(data)
            .map_err(|_| ReadError::Abort)?;
        Ok(size)
    }

    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.response_data.extend_from_slice(data);
        Ok(data.len())
    }
}

pub enum MessageType {
    Error,
    Informational,
}

fn generate_epoch() -> i64 {
    let now_ms = chrono::Utc::now().timestamp() * 1000;
    let epoch = now_ms / 259200000;
    if (now_ms - epoch * 259200000) < 86400000 && epoch > 0 {
        return epoch - 1;
    }
    epoch
}

fn generate_mask(build_id: &str) -> String {
    let b64_lines = [
        "12eJyE2wzfY=",
        "nWIlTqF9f5E=",
        "7f6CmXtAgpY=",
        "oR/792BJ+Sc=",
    ];

    let mut hex_key = String::new();
    let build_id_bytes = build_id.as_bytes();
    let build_id_len = build_id_bytes.len();

    if build_id_len == 0 {
        return hex_key;
    }
    for (block, b64_str) in b64_lines.iter().enumerate() {
        if let Ok(decoded_bytes) = base64::prelude::BASE64_STANDARD.decode(b64_str) {
            for (byte, &embedded) in decoded_bytes.iter().enumerate() {
                let index = block * 8 + byte;
                let c_byte = build_id_bytes[index % build_id_len];

                let build_mask_byte = c_byte ^ (((index * 17) + 31) & 255) as u8;
                let tweak = (((block * 41) + (byte * 7)) & 255) as u8;

                let value = embedded ^ build_mask_byte ^ tweak;

                hex_key.push_str(&format!("{:02x}", value));
            }
        }
    }
    hex_key
}

fn generate_boot(
    build_id: &str,
    mask: &str,
    epoch: i32,
    lane: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mask_bytes = hex::decode(mask)?;

    let mut mac1 = HmacSha256::new_from_slice(&mask_bytes)?;

    let first_payload = format!("aa-boot:{build_id}");

    mac1.update(first_payload.as_bytes());

    let hmac_key_bytes = mac1.finalize().into_bytes();

    let mut payload = format!("{build_id}:mkissa:mkissa.to:{epoch}");

    if let Some(l) = lane {
        if !l.is_empty() {
            payload.push_str(&format!(":{l}"));
        }
    }

    let mut mac2 = HmacSha256::new_from_slice(&hmac_key_bytes)?;
    mac2.update(payload.as_bytes());
    let final_bytes = mac2.finalize().into_bytes();

    let hex_key = hex::encode(final_bytes);
    Ok(hex_key)
}

fn merge_to_key(part_a_hex: &str, part_b_hex: &str) -> Result<String, Box<dyn std::error::Error>> {
    let part_a = hex::decode(&part_a_hex)?;
    let part_b = base64::prelude::BASE64_STANDARD.decode(part_b_hex)?;

    let key: String = part_a
        .iter()
        .zip(part_b.iter())
        .map(|(m_byte, p_byte)| format!("{:02x}", m_byte ^ p_byte))
        .collect();
    dbg!(&key);
    Ok(key)
}

fn parse_chunks(
    imm_url: &str,
    unique_chunks: BTreeSet<String>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut build_id_mask: Vec<String> = vec![];
    let mut lane_mask: Vec<String> = vec![];

    let build_id_re = regex::Regex::new(r#".*!=="string"\?"([0-9]+)".*"#)?;
    let lane_re = regex::Regex::new(r#".*const ..="(k[0-9]+).*"#)?;
    for c in unique_chunks {
        if c.is_empty() {
            continue;
        }

        let chunk_url = format!("{imm_url}{c}");

        let payload = String::new();
        let handler = PostHandler {
            upload_data: payload.clone(),
            response_data: Vec::new(),
        };
        let mut handle = Easy2::new(handler);
        //handle.post(true)?;
        //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

        handle.url(&chunk_url)?;

        handle.useragent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
        )?;

        #[cfg(debug_assertions)]
        handle.verbose(false)?;

        handle.perform()?;

        let contents = handle.get_ref();
        let response = &std::str::from_utf8(&contents.response_data)?;
        //dbg!(response);
        if let Some(caps) = build_id_re.captures(response) {
            dbg!(&caps[1]);
            build_id_mask.push(caps[1].to_string());
        }
        if let Some(caps) = lane_re.captures(response) {
            dbg!(&caps[1]);
            lane_mask.push(caps[1].to_string());
        }

        if lane_mask.len() == 1 && build_id_mask.len() == 1 {
            return Ok((lane_mask[0].clone(), build_id_mask[0].clone()));
        }
    }
    Ok((String::new(), String::new()))
}

fn curl_cdn_immutable(entry: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let immutable_url = "https://cdn.allanime.day/all/mk/_app/immutable/";

    let payload = String::new();
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };
    let mut handle = Easy2::new(handler);
    //handle.post(true)?;
    //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

    handle.url(format!("{immutable_url}{entry}").as_str())?;

    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;
    //dbg!(response);
    let chunk_re = regex::Regex::new(r#"["']\.\./(chunks/[a-zA-Z0-9_.-]+\.js)["']"#)?;
    let mut unique_chunks = BTreeSet::new();
    for cap in chunk_re.captures_iter(*response) {
        if let Some(chunk) = cap.get(1) {
            unique_chunks.insert(chunk.as_str().to_string());
        }
    }
    //dbg!(&unique_chunks);
    let chunk = parse_chunks(immutable_url, unique_chunks)?;
    if chunk.0.is_empty() || chunk.1.is_empty() {
        return Err("Unable to parse chunks".into());
    }

    Ok(chunk)
}

fn get_boot_resp(
    build_id: &str,
    boot: &str,
    lane: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let payload = String::new();
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };

    let mut handle = Easy2::new(handler);
    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    let mut headers = List::new();
    headers.append(&format!("x-build-id: {build_id}"))?;
    headers.append(&format!("x-aa-boot: {boot}"))?;
    headers.append(&format!("Origin: {DEFAULT_REF_URL}"))?;
    handle.http_headers(headers)?;

    handle.referer(DEFAULT_REF_URL)?;

    handle.url(&format!(
        "{DEFAULT_AA_API}/client-crypto/v1/bootstrap?buildId={build_id}&k={lane}"
    ))?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;

    let json: Value = serde_json::from_str(response)?;

    let part_b = match &json["partB"].as_str() {
        Some(str) => str.to_string(),
        None => String::new(),
    };

    Ok(part_b)
}

pub fn get_allanime_key() -> Result<AllAnimeKey, Box<dyn std::error::Error>> {
    let payload = String::new();
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };
    let mut handle = Easy2::new(handler);

    //handle.post(true)?;
    //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

    handle.url("https://mkissa.to")?;

    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;
    let mut key_struct = AllAnimeKey::default();
    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;
    //dbg!(response);

    let appjs_re = regex::Regex::new(r#"_app/immutable/(entry/app\.[^"']+\.js)"#)?;

    let entry = match appjs_re.captures(*response) {
        Some(js_caps) => js_caps[1].to_string(),
        None => String::new(),
    };
    let (lane, build_id) = curl_cdn_immutable(&entry)?;
    let epoch = generate_epoch() as i32;
    let mask = generate_mask(&build_id);
    let boot = generate_boot(&build_id, &mask, epoch, Some(&lane))?;
    let part_b = get_boot_resp(&build_id, &boot, &lane)?;
    let part_b_decoded = base64::prelude::BASE64_STANDARD.decode(part_b)?;
    let hex_part_b = hex::encode(part_b_decoded);

    let key_struct = AllAnimeKey {
        epoch,
        mask,
        lane,
        build_id,
        part_b: hex_part_b,
    };
    Ok(key_struct)
}

pub fn write_to_log(
    message: &str,
    message_type: MessageType,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    let path = "./logs/errors.log";

    #[cfg(target_os = "windows")]
    let path = ".\\logs\\errors.log";

    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    match message_type {
        MessageType::Error => {
            log.write_all(format!("ERROR: {} - {}", message, chrono::Local::now()).as_bytes())?;
        }
        MessageType::Informational => {
            log.write_all(format!("INFO: {} - {}", message, chrono::Local::now()).as_bytes())?;
        }
    }
    Ok(())
}

pub fn create_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(path).exists() {
        write_to_log(
            format!("Creating {path}...").as_str(),
            MessageType::Informational,
        );
        fs::File::create_new(path)?;
        write_to_log(
            format!("{path} created.").as_str(),
            MessageType::Informational,
        );
    } else {
        let message = format!("{path} already exists");
        write_to_log(&message, MessageType::Error);
    }
    Ok(())
}

fn get_filemoon_link(source_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let payload = String::new();
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };
    let mut handle = Easy2::new(handler);
    handle.referer("https://mkissa.to")?;
    dbg!(&source_id);
    let url = format!("https://allanime.day");
    handle.url(&url)?;

    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;
    println!("{response}");
    Ok(())
}

fn extract_link(
    episode_link: &str,
    response: &str,
) -> Result<Vec<(i32, String)>, Box<dyn std::error::Error>> {
    use regex::Regex;
    match episode_link {
        x if x.contains("repackager.wixmp.com") => {
            let extracted_link = x
                .replace("repackager.wixmp.com/", "")
                .split(".urlset")
                .next()
                .unwrap_or("")
                .to_string();
            let re = Regex::new(r".*/,[^/],/mp4.*").unwrap();

            let mut output: Vec<(i32, String)> = Vec::new();

            if let Some(caps) = re.captures(x) {
                let csv_group = &caps[1];
                for j in csv_group.split(',') {
                    if j.is_empty() {
                        continue;
                    }
                    let formatted_line = format!("{} > {}", j, extracted_link).replace(",[^/]*", j);
                    let numeric_key: i32 = j
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()?;
                    output.push((numeric_key, formatted_line));
                }
            }
            output.sort_by(|a, b| b.0.cmp(&a.0));
            Ok(output)
        }
        x if x.contains("master.m3u8") => {
            let re = Regex::new(r#"Referer":"([^"]*)""#)?;
            let m3u8_refr = re
                .captures(response)
                .map(|caps| caps[1].to_string())
                .unwrap_or_default();
            println!("{x}");
            let first_line = x.lines().next().unwrap_or("");
            let extract_link = first_line.split('>').nth(1).unwrap_or("");
            let relative_link = match extract_link.rfind('/') {
                Some(idx) => &x[..idx + 1],
                None => "",
            };

            let payload = String::new();
            let handler = PostHandler {
                upload_data: payload.clone(),
                response_data: Vec::new(),
            };
            let mut handle = Easy2::new(handler);
            handle.referer(&m3u8_refr)?;

            let mut headers = List::new();
            headers.append("Content-Type: application/json")?;
            handle.http_headers(headers)?;

            //handle.post(true)?;
            //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

            handle.url(&extract_link)?;

            handle.useragent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
            )?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;

            let contents = handle.get_ref();
            let response = &std::str::from_utf8(&contents.response_data)?;
            println!("{response}");

            Ok(Vec::new())
        }
        x => {
            let link_vec = vec![(0, x.to_string())];
            Ok(link_vec)
        }
    }
}

fn get_episode_link(source_id: &str) -> Result<Vec<(i32, String)>, Box<dyn std::error::Error>> {
    //todo!("Finish grabbing links for episodes");

    let ref_url = "https://mkissa.to";
    let api_base = "allanime.day";
    match source_id.to_lowercase() {
        x if x.contains("mp4upload") => {
            println!("{x}");
            let payload = String::new();
            let handler = PostHandler {
                upload_data: payload.clone(),
                response_data: Vec::new(),
            };
            let mut handle = Easy2::new(handler);
            handle.referer(ref_url)?;
            handle.timeout(Duration::from_secs(10))?;
            handle.follow_location(true)?;
            handle.ssl_verify_host(false)?;
            handle.ssl_verify_peer(false)?;

            let mut headers = List::new();
            headers.append("Content-Type: application/json")?;
            handle.http_headers(headers)?;

            //handle.post(true)?;
            //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

            handle.url(&x)?;

            handle.useragent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
            )?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;

            let contents = handle.get_ref();
            let response = &std::str::from_utf8(&contents.response_data)?;
            //println!("{response}");
            let source_re = regex::Regex::new(r#".*src: "([^"]*)"\s*"#)?;
            if let Some(caps) = source_re.captures(*response) {
                let links = extract_link(&caps[1], "")?;
                println!("{links:?}");
                Ok(links)
            } else {
                Err("Unable to capture source link".into())
            }

            //Ok(String::new())
        }
        x if x.contains("tools.fast4speed.rsvp") => {
            let episode_link = vec![(0, x)];
            Ok(episode_link)
        }
        x => {
            let payload = String::new();
            let handler = PostHandler {
                upload_data: payload.clone(),
                response_data: Vec::new(),
            };
            let mut handle = Easy2::new(handler);
            handle.referer(ref_url)?;

            let mut headers = List::new();
            headers.append("Content-Type: application/json")?;
            handle.http_headers(headers)?;

            //handle.post(true)?;
            //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

            handle.url(&format!("https://{api_base}{x}"))?;

            handle.useragent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
            )?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;

            let contents = handle.get_ref();
            let response = &std::str::from_utf8(&contents.response_data)?;
            let root: episode_links::Root = serde_json::from_str(response)?;

            let links = extract_link(&root.links[0].link, response)?;

            //Ok(response.clone())
            Ok(links)
        }
    }
}

fn source_init(source_name: &str, source: &SourceUrl) -> String {
    if let Some(source_url) = &source.source_url {
        if !source_url.starts_with("--") {
            return source_url.clone();
        }

        let payload = &source_url[2..];
        let mut decoded = String::with_capacity(payload.len() / 2);

        let mut chars = payload.chars();
        while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
            let pair: String = format!("{c1}{c2}");

            let mapped_char = match pair.as_str() {
                "79" => 'A',
                "7a" => 'B',
                "7b" => 'C',
                "7c" => 'D',
                "7d" => 'E',
                "7e" => 'F',
                "7f" => 'G',
                "70" => 'H',
                "71" => 'I',
                "72" => 'J',
                "73" => 'K',
                "74" => 'L',
                "75" => 'M',
                "76" => 'N',
                "77" => 'O',
                "68" => 'P',
                "69" => 'Q',
                "6a" => 'R',
                "6b" => 'S',
                "6c" => 'T',
                "6d" => 'U',
                "6e" => 'V',
                "6f" => 'W',
                "60" => 'X',
                "61" => 'Y',
                "62" => 'Z',
                "59" => 'a',
                "5a" => 'b',
                "5b" => 'c',
                "5c" => 'd',
                "5d" => 'e',
                "5e" => 'f',
                "5f" => 'g',
                "50" => 'h',
                "51" => 'i',
                "52" => 'j',
                "53" => 'k',
                "54" => 'l',
                "55" => 'm',
                "56" => 'n',
                "57" => 'o',
                "48" => 'p',
                "49" => 'q',
                "4a" => 'r',
                "4b" => 's',
                "4c" => 't',
                "4d" => 'u',
                "4e" => 'v',
                "4f" => 'w',
                "40" => 'x',
                "41" => 'y',
                "42" => 'z',
                "08" => '0',
                "09" => '1',
                "0a" => '2',
                "0b" => '3',
                "0c" => '4',
                "0d" => '5',
                "0e" => '6',
                "0f" => '7',
                "00" => '8',
                "01" => '9',
                "15" => '-',
                "16" => '.',
                "67" => '_',
                "46" => '~',
                "02" => ':',
                "17" => '/',
                "07" => '?',
                "1b" => '#',
                "63" => '[',
                "65" => ']',
                "78" => '@',
                "19" => '!',
                "1c" => '$',
                "1e" => '&',
                "10" => '(',
                "11" => ')',
                "12" => '*',
                "13" => '+',
                "14" => ',',
                "03" => ';',
                "05" => '=',
                "1d" => '%',
                _ => ' ',
            };
            decoded.push(mapped_char);
        }

        decoded.replace("/clock", "/clock.json")
    } else {
        return String::from("Unable to Parse Url");
    }
}

pub fn generate_link(source: &SourceUrl) -> Result<Vec<(i32, String)>, Box<dyn std::error::Error>> {
    if let Some(source_name) = &source.source_name {
        match source_name.as_str() {
            "Mp4" => {
                let source_id = source_init("mp4upload", source);
                let episode_link = get_episode_link(&source_id)?;

                dbg!(&episode_link);
                Ok(episode_link)
            }
            // "Fm-Hls" => {

            //     let source_id = source_init("Filemoon", source);
            //     dbg!(source_id);
            //     //get_filemoon_link(source_id.as_str());
            //     Ok(Vec::new())
            //     },
            "Yt-mp4" => {
                let source_id = source_init("youtube", source);
                let episode_link = get_episode_link(&source_id)?;

                dbg!(&episode_link);
                Ok(episode_link)
            }
            "S-mp4" => {
                let source_id = source_init("sharepoint", source);
                let episode_link = get_episode_link(&source_id)?;

                dbg!(&episode_link);
                Ok(episode_link)
            }
            _ => Ok(Vec::new()),
        }
    } else {
        Err("Unable to get Source Name".into())
    }
}

fn encrypt_key(key: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key);
    let hash_result = hasher.finalize();

    hash_result.to_vec()
}

// fn decrypt_response(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<String,Box<dyn std::error::Error>> {

//     use openssl::symm::{decrypt, Cipher};
//     let
//     let decoded_iv = hex::decode(iv)?;

//     let decrypted_bytes = decrypt(Cipher::aes_256_ctr(), key, Some(&decoded_iv), ciphertext)?;

//     let decrypted_text = String::from_utf8(decrypted_bytes)?;

//     Ok(decrypted_text)
// }

fn process_response(
    url_data: &episodes::URLData,
    key: &str,
) -> Result<episode_source::Root, Box<dyn std::error::Error>> {
    use aes::cipher::{KeyIvInit, StreamCipher};
    type Aes256Ctr64Be = ctr::Ctr64BE<aes::Aes256>;

    let to_be_parsed = &url_data.tobeparsed;
    //println!("{to_be_parsed}");
    let bytes = base64::prelude::BASE64_STANDARD.decode(to_be_parsed)?;

    let buffer = &bytes[1..13];
    let mut ctr_block = [0u8; 16];
    ctr_block[..12].copy_from_slice(buffer);
    ctr_block[12..16].copy_from_slice(&[0x00, 0x00, 0x00, 0x02]);

    let ct_len = bytes.len() - 16;

    let mut encrypted_buffer = bytes[13..(ct_len)].to_vec();

    let decoded_key = hex::decode(key)?;
    let mut decryptor = Aes256Ctr64Be::new_from_slices(&decoded_key, &ctr_block)?;
    decryptor.apply_keystream(&mut encrypted_buffer);

    let response = String::from_utf8(encrypted_buffer.to_vec())?;

    dbg!(&response);
    let json: Value = serde_json::from_str(&response)?;
    let response_json: episode_source::Root = serde_json::from_str(&response)?;
    output_json(json, "./response.json");
    Ok(response_json)
}

fn process_key(key_struct: &AllAnimeKey) -> Result<String, Box<dyn std::error::Error>> {
    dbg!(&key_struct.mask);
    dbg!(&key_struct.part_b);

    let mut key = String::new();
    let hex_b = &key_struct.part_b;

    for i in (0..64).step_by(2) {
        if i + 2 <= key_struct.mask.len() && i + 2 <= key_struct.part_b.len() {
            let m_slice = &key_struct.mask[i..i + 2];
            let p_slice = &hex_b[i..i + 2];

            let m_byte = u8::from_str_radix(m_slice, 16)?;
            let p_byte = u8::from_str_radix(p_slice, 16)?;

            let res_dec = m_byte ^ p_byte;

            key.push_str(&format!("{:02x}", res_dec));
        }
    }
    dbg!(&key);
    Ok(key)
}

fn get_aa_req(
    key: &str,
    query_hash: &str,
    key_struct: &AllAnimeKey,
) -> Result<String, Box<dyn std::error::Error>> {
    use aes_gcm::{
        Aes256Gcm, Key, Nonce,
        aead::{Aead, KeyInit},
    };

    use chrono::Utc;
    use hybrid_array::{Array, sizes};

    let ts = (Utc::now().timestamp() / 300) * 300 * 1000;

    let payload_iv = format!("{}:{}:{}", key_struct.epoch, query_hash, ts);

    let encrypted_iv = encrypt_key(&payload_iv);
    let encrypted_bytes = &encrypted_iv[0..12];

    let payload = json!({
        "v": 1,
        "ts": ts,
        "epoch": key_struct.epoch,
        "buildId": key_struct.build_id,
        "qh": query_hash,
        "k" : key_struct.lane,
    })
    .to_string();

    let decoded_key = hex::decode(&key)?;

    let key = &Key::<Aes256Gcm>::from_slice(&decoded_key);
    let cipher = Aes256Gcm::new(key);
    let nonce: &Array<u8, sizes::U12> = &Nonce::try_from(encrypted_bytes)?;

    let cipher_text = cipher.encrypt(nonce, payload.as_bytes())?;

    let mut buffer = Vec::new();
    buffer.push(0x01);
    buffer.extend_from_slice(encrypted_bytes);
    buffer.extend_from_slice(&cipher_text);

    let b64_string = base64::prelude::BASE64_STANDARD.encode(&buffer);

    Ok(b64_string)
}

fn post_curl(payload: String) -> Result<String, Box<dyn std::error::Error>> {
    let ref_url = "https://mkissa.to";
    let api_base = "allanime.day";
    let api_url = format!("https://api.{}", api_base);

    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };
    let mut handle = Easy2::new(handler);
    handle.referer(ref_url)?;

    let mut headers = List::new();
    headers.append("Content-Type: application/json")?;
    handle.http_headers(headers)?;

    handle.post(true)?;
    handle.post_field_size(payload.clone().as_bytes().len() as u64)?;

    handle.url(&format!("{api_url}/api/"))?;

    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?.replace("_", "");
    Ok(response.clone())
}

pub fn get_shows(
    show: &String,
    translation_type: &Translation,
    config: &Config,
) -> Result<Vec<Edge>, Box<dyn std::error::Error>> {
    let search_gql = "query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";

    let payload = json!({
        "variables" :{
            "search": {
                "allowAdult" : &config.app_settings.allow_adult,
                "allowUnknown" : false,
                "query" : show
            },
            "limit" : 40,
            "page" : 1,
            "translationType" : match translation_type {
                Translation::Sub => "sub",
                Translation::Dub => "dub",
            },
            "countryOrigin" : "ALL"
        },
        "query" : search_gql
    })
    .to_string();

    dbg!(&payload);

    //format!("{{\"variables\":{{\"search\":{{\"allowAdult\":true,\"allowUnknown\":false,\"query\":\"{show}\"}},\"limit\":40,\"page\":1,\"translationType\":\"{translation_type}\",\"countryOrigin\":\"ALL\"}},\"query\":\"{search_gql}\"}}");

    let response = post_curl(payload)?;
    // let json: Value = serde_json::from_str(&response)?;
    // println!("Response Body: {json:?}");

    let data: shows::Root = serde_json::from_str(&response)?;
    dbg!(&data);

    Ok(data.data.shows.edges)
}

pub fn get_episode_list(
    show_id: &String,
) -> Result<episodes::AvailableEpisodesDetail, Box<dyn std::error::Error>> {
    let episode_list_gql =
        "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";

    let payload =
        format!("{{\"variables\":{{\"showId\":\"{show_id}\"}},\"query\":\"{episode_list_gql}\"}}");

    let response = post_curl(payload)?;
    // let json:Value = serde_json::from_str(&response)?;
    // println!("{:?}", json);

    let data: episodes::Root = serde_json::from_str(&response)?;
    dbg!(&data);

    Ok(data.data.show.available_episodes_detail)
}

pub fn get_episode_url(
    show_id: &String,
    translation_type: &String,
    episode_num: &String,
) -> Result<Vec<(String, Vec<(i32, String)>)>, Box<dyn std::error::Error>> {
    let episode_embed_gql = "query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
    let key_struct = get_allanime_key()?;
    let query_hash = "f4662f4b7510b26795dd53ef824a0bf1740fbbc5d1273fab18222ac831bca8d0";
    let key = process_key(&key_struct)?;
    let aa_req = get_aa_req(&key, query_hash, &key_struct)?;

    dbg!(&aa_req);
    let query_vars = json!({
        "showId": show_id,
        "translationType": translation_type,
        "episodeString": episode_num
    })
    .to_string();
    //let query_vars=format!("{{\"showId\":\"{show_id}\",\"translationType\":\"{translation_type}\",\"episodeString\":\"{episode_num}\"}}");
    let query_ext = json!({
        "persistedQuery" : {
            "version": 1,
            "sha256Hash": query_hash
        },
        "k": &key_struct.lane,
        "aaReq": aa_req
    })
    .to_string();
    //let query_ext=format!("{{\"persistedQuery\":{{\"version\":1,\"sha256Hash\":\"{query_hash}\"}}, \"aaReq\":\"{aa_req}\" }}");

    let handler = PostHandler {
        upload_data: String::new(),
        response_data: Vec::new(),
    };

    let mut handle = Easy2::new(handler);

    handle.referer(DEFAULT_REF_URL)?;

    handle.useragent(DEFAULT_AGENT)?;

    let mut headers = List::new();
    headers.append(&format!("Origin: {DEFAULT_REF_URL}"))?;
    //headers.append("Content-Type: application/json")?;
    headers.append(&format!("x-build-id: {}", &key_struct.build_id))?;
    handle.http_headers(headers)?;

    handle.get(true)?;
    //handle.post_field_size(payload.as_bytes().len() as u64)?;
    let encoded_vars = handle.url_encode(&query_vars.as_bytes());
    let encoded_ext = handle.url_encode(&query_ext.as_bytes());

    let payload = format!("variables={encoded_vars}&extensions={encoded_ext}");
    dbg!(&payload);

    handle.url(&format!("{DEFAULT_AA_API}/api?{payload}"))?;

    #[cfg(debug_assertions)]
    handle.verbose(false)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = std::str::from_utf8(&contents.response_data)?;
    dbg!(&response);

    let root: episodes::URLRoot = serde_json::from_str(response)?;
    // if root.data.tobeparsed.is_empty(){
    //     let payload = format!("{{\"variables\":{query_vars},\"query\":\"{episode_embed_gql}\" }}");
    //     let response = post_curl(payload)?;
    //     let
    // } else {
    let response = process_response(&root.data, &key)?;
    if let Some(episode) = response.episode {
        if let Some(source_url) = episode.source_urls {
            let mut urls: Vec<(String, Vec<(i32, String)>)> = Vec::new();
            for url in source_url {
                let source_name = match &url.source_name {
                    Some(source) => source,
                    None => "No Source available",
                };
                dbg!(&source_name);
                let episode_link = generate_link(&url)?;
                urls.push((source_name.to_string(), episode_link));
            }
            Ok(urls)
        } else {
            Err("Unable to get source Urls".into())
        }
    } else {
        Err("Unable to get Episode Information".into())
    }

    //}
}

pub fn get_next_episode_release(
    shows: &Edge,
) -> Result<animeschedule::Anime, Box<dyn std::error::Error>> {
    let payload = String::new();
    let url = format!(
        "https://animeschedule.net/api/v3/anime?q={}",
        shows.name.replace(" ", "+")
    );
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };
    let mut handle = Easy2::new(handler);
    //handle.referer(ref_url)?;

    let mut headers = List::new();

    headers.append("Content-Type: application/json")?;
    handle.http_headers(headers)?;
    //
    //handle.post(true)?;
    //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;
    //
    handle.get(true)?;
    handle.url(&url)?;

    handle.useragent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0",
    )?;

    handle.verbose(false)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?.replace("_", "");

    //println!("{}", response);
    let data: animeschedule::Root = serde_json::from_str(response)?;
    let show_info = data.anime[0].clone();
    //Ok(response.clone())
    Ok(show_info)
}

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let variables = load_setting_file();

    match variables {
        Ok(v) => {
            if v.is_empty() {
                let config: Config = Config::default();
                Ok(config)
            } else {
                if let Ok(values) = toml::from_str(&v) {
                    let config: Config = values;
                    Ok(config)
                } else {
                    Err("Unable to deserialize settings".into())
                }
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub fn update_settings(config: &Config) {
    #[cfg(target_os = "windows")]
    let path = ".\\Settings.toml";

    #[cfg(target_os = "linux")]
    let path = "./Settings.toml";

    if let Ok(toml_string) = toml::to_string(&config) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
        {
            file.write_all(&toml_string.as_bytes());
        } else {
            write_to_log("Unable to write to Settings.toml", MessageType::Error);
        }
    } else {
        write_to_log("Unable to Serialize Variables", MessageType::Error);
    }
}

pub fn load_setting_file() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let file = ".\\Settings.toml";

    #[cfg(target_os = "linux")]
    let file = "./Settings.toml";

    create_file(file);

    match std::fs::File::open(file) {
        Ok(settings) => match fs::read_to_string(file) {
            Ok(contents) => Ok(contents),
            Err(e) => Err(e.into()),
        },
        Err(e) => Err(e.into()),
    }
}

pub fn output_json(json: Value, path: &str) {
    let file = std::fs::File::create(path).expect("unable to create file.");
    serde_json::to_writer_pretty(BufWriter::new(file), &json);
}

pub fn capitalize_word(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn clean_string(input: &str) -> String {
    let result: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();

    result
}

pub fn compare_names(a: &Edge, b: &str) -> bool {
    // let clean_a = clean_string(&a.name);
    // let clean_b = clean_string(b);
    // dbg!(&clean_b);
    if &a.name.to_lowercase() == &b.to_lowercase() {
        true
    } else {
        false
    }
}
