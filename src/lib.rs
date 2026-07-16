use std::{
    error::Error, io::{Read, Write, stdout}, process::Command, string, sync::mpsc::{Receiver, Sender}

};


use json::{parse, short};
use urlencoding::encode;
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

fn encrypt_key(key: &str) -> Vec<u8>{
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hash_result = hasher.finalize();
   
    hash_result.to_vec()
}

fn decrypt_response(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<String,Box<dyn std::error::Error>> {
   
   
    use openssl::symm::{decrypt, Cipher};
    let decoded_iv = hex::decode(iv)?;
    
    let decrypted_bytes = decrypt(Cipher::aes_256_ctr(), key, Some(&decoded_iv), ciphertext)?;
   
    let decrypted_text = String::from_utf8(decrypted_bytes)?;
    
    Ok(decrypted_text)
}

fn process_response(url_data: &episodes::URLData)-> Result<episode_source::Root,Box<dyn std::error::Error>>{
    use base64::{Engine as _, alphabet, engine::{self, general_purpose}};
    let to_be_parsed = &url_data.tobeparsed;
    //println!("{to_be_parsed}");
    let bytes = general_purpose::STANDARD.decode(to_be_parsed)?;
    
    let buffer = &bytes[1..=12];
    let ct_len = bytes.len()-16;

    let encrypted_buffer = &bytes[13..ct_len];

    let key= encrypt_key("Xot36i3lK3:v1");
    //println!("{}", key);
    let iv = hex::encode(buffer);
    let ctr = format!("{iv}00000002");
    //println!("{}", iv);
    //println!("{}", ctr);
    let response = decrypt_response(&key, ctr.as_bytes(), encrypted_buffer)?;
    
    let response_json: episode_source::Root= serde_json::from_str(&response)?;
    
    Ok(response_json)
}


fn post_curl(payload: String) ->Result<String, Box<dyn std::error::Error>>{
    
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

    let payload = format!("{{\"variables\":{{\"search\":{{\"allowAdult\":true,\"allowUnknown\":false,\"query\":\"yani neko\"}},\"limit\":40,\"page\":1,\"translationType\":\"sub\",\"countryOrigin\":\"ALL\"}},\"query\":\"{search_gql}\"}}");

    let response = post_curl(payload)?;
    // let json: Value = serde_json::from_str(&response)?; 
    // println!("Response Body: {json:?}");

    let data:shows::Root = serde_json::from_str(&response)?;
    println!("Response Body: {data:?}");

    
    Ok(data.data.shows.edges)
}

pub fn get_episode_list(show_id: &String) -> Result<episodes::AvailableEpisodesDetail, Box<dyn std::error::Error>>{
    let episode_list_gql = "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";

    let payload = format!("{{\"variables\":{{\"showId\":\"{show_id}\"}},\"query\":\"{episode_list_gql}\"}}");

    let response = post_curl(payload)?;
    // let json:Value = serde_json::from_str(&response)?;
    // println!("{:?}", json);

    let data:episodes::Root = serde_json::from_str(&response)?;
    println!("{:?}", data);


    Ok(data.data.show.available_episodes_detail)

}

pub fn get_episode_url(show_id: &String, translation_type: &String,  episode_num: &String)->Result<Vec<episode_source::SourceUrl>, Box<dyn std::error::Error>>{

    let episode_embed_gql="query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
    
    let query_hash = "d405d0edd690624b66baba3068e0edc3ac90f1597d898a1ec8db4e5c43c00fec";
    let query_vars=format!("{{\"showId\":\"{show_id}\",\"translationType\":\"{translation_type}\",\"episodeString\":\"{episode_num}\"}}");
    let query_ext=format!("{{\"persistedQuery\":{{\"version\":1,\"sha256Hash\":\"{query_hash}\"}}}}");
    let payload = format!("variables={}&extensions={}", encode(&query_vars), encode(&query_ext));


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
    
    handle.get(true)?;
    //handle.post_field_size(payload.as_bytes().len() as u64)?;
    
    
    handle.url(&format!("{api_url}/api?{payload}"))?;

    handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;

    handle.verbose(false)?;

    handle.perform()?;
    

    let contents = handle.get_ref();
    let response =&std::str::from_utf8(&contents.response_data)?.replace("_", "");
    let root: episodes::URLRoot = serde_json::from_str(response)?;
    // if root.data.tobeparsed.is_empty(){
    //     let payload = format!("{{\"variables\":{query_vars},\"query\":\"{episode_embed_gql}\" }}");
    //     let response = post_curl(payload)?;
    //     let
    // } else {
    let response = process_response(&root.data)?;
    let source_urls = response.episode.source_urls;
    Ok(source_urls)
//}

}

pub fn get_next_episode_release(shows: &Edge) -> Result<animeschedule::Anime, Box<dyn std::error::Error>>{
    
    let payload = String::new();
    let url = format!("https://animeschedule.net/api/v3/anime?q={}",shows.name.replace(" ", "+"));
    let handler = PostHandler{
        upload_data: payload.clone(),
        response_data: Vec::new()
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
 
    handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;
 
    handle.verbose(false)?;
 
    handle.perform()?;

    let contents = handle.get_ref();
    let response =&std::str::from_utf8(&contents.response_data)?.replace("_", "");
    
    //println!("{}", response);
    let data: animeschedule::Root = serde_json::from_str(response)?;
    let show_info = data.anime[0].clone();
    //Ok(response.clone())
     Ok(show_info)
}
