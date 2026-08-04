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
use tokio::{io::repeat, process};
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

pub async fn find_player(player: &str) -> Result<String, Box<dyn Error>> {
    let cmd = process::Command::new("which").arg(player).output().await?;
    let location = String::from_utf8(cmd.stdout)?.trim().to_string();
    dbg!(&location);
    Ok(location)
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
