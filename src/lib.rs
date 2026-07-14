use std::{
    io::{stdout, Write, Read},
    sync::mpsc::{Sender, Receiver},
    error::Error,
    process::Command

};

use json::parse;

use serde_json::Value;
use curl::easy::{Handler, Easy2, WriteError, ReadError, List};

mod structs;

pub use structs::*;

#[derive(PartialEq, Debug)]
pub enum Translation{
    Sub,
    Dub,
    Raw
}


struct PostHandler{
    upload_data: String,
    response_data: Vec<u8>
}

impl Handler for PostHandler{
    fn read(&mut self, data: &mut [u8])->Result<usize, ReadError>{
        let size = self.upload_data.as_bytes().read(data).map_err(|_| ReadError::Abort)?;
        Ok(size)    
    }

    fn write(&mut self, data: &[u8]) ->Result <usize, WriteError>{
        self.response_data.extend_from_slice(data);
        Ok(data.len())
    }
}

fn post_curl(payload: String, show_id: Option<&String>) ->Result<String, Box<dyn std::error::Error>>{
    
    let ref_url = "https://youtu-chan.com";
    let api_base =  "allanime.day";
    let api_url = format!("https://api.{}", api_base);

    let handler = PostHandler{
        upload_data: payload.clone(),
        response_data: Vec::new()
    };
    let mut handle = Easy2::new(handler);
    handle.referer(ref_url)?;

    let mut headers = List::new();
    headers.append("Content-Type: application/json")?;
    handle.http_headers(headers)?;
    
    handle.post(true)?;
    handle.post_field_size(payload.clone().as_bytes().len() as u64)?;
    
    
    handle.url(&format!("{api_url}/api/"))?;

    handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;

    handle.verbose(true)?;

    handle.perform()?;
    

    let contents = handle.get_ref();
    let response =&std::str::from_utf8(&contents.response_data)?.replace("_", "");
    Ok(response.clone())
}


pub fn get_shows() -> Result<Vec<Edge>, Box<dyn std::error::Error>>{

    let search_gql="query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";

    let payload = format!("{{\"variables\":{{\"search\":{{\"allowAdult\":true,\"allowUnknown\":false,\"query\":\"yani\"}},\"limit\":40,\"page\":1,\"translationType\":\"sub\",\"countryOrigin\":\"ALL\"}},\"query\":\"{search_gql}\"}}");

    let response = post_curl(payload, None)?;
    // let json: Value = serde_json::from_str(&response)?; 
    // println!("Response Body: {json:?}");

    let data:shows::Root = serde_json::from_str(&response)?;
    println!("Response Body: {data:?}");

    
    Ok(data.data.shows.edges)
}

pub fn get_episode_list(show_id: &String) -> Result<episodes::AvailableEpisodesDetail, Box<dyn std::error::Error>>{
    let episode_list_gql = "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";

    let payload = format!("{{\"variables\":{{\"showId\":\"{show_id}\"}},\"query\":\"{episode_list_gql}\"}}");

    let response = post_curl(payload, Some(show_id))?;
    // let json:Value = serde_json::from_str(&response)?;
    // println!("{:?}", json);

    let data:episodes::Root = serde_json::from_str(&response)?;
    println!("{:?}", data);


    Ok(data.data.show.available_episodes_detail)

}

pub fn get_episode_url(show_id: &String, translation_type: Translation,  episode_num: i16){
    let episode_embed_gql="query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
    let mut translation = String::new();
    match translation_type{
        Translation::Sub => translation = "sub".to_string(),
        Translation::Dub => translation = "dub".to_string(),
        Translation::Raw => translation = "raw".to_string(),
    }
    println!("{translation}");
    //let query_vars=format!("{{\"showId\":\"{}\",\"translationType\":\"{}\",\"episodeString\":\"{episode_num}\"}}", show_id, translation_type, );
    //let query_ext="{\"persistedQuery\":{\"version\":1,\"sha256Hash\":\"$allanime_query_hash\"}, \"aaReq\":\"$(get_aa_req)\"}";
}
