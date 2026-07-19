use std::{
    error::Error, io::{Read, Write, stdout}, process::Command, string, sync::mpsc::{Receiver, Sender}, time::Duration

};


use base64::Engine;
use hybrid_array::{ArraySize, typenum};
use serde_json::json;
use openssl_sys::EVP_MAC_CTX_new;
use tokio::io::repeat;
use urlencoding::encode;
use curl::easy::{Handler, Easy2, WriteError, ReadError, List};

mod structs;

pub use structs::*;

#[derive(PartialEq, Debug, Clone, Copy)]
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


fn get_filemoon_link(){
    
}

fn extract_link(episode_link: &str, response: &str)->Result<Vec<(i32, String)>, Box<dyn std::error::Error>>{
    use regex::Regex;
    match episode_link{
        x if x.contains("repackager.wixmp.com")=>{
            let extracted_link = x.replace("repackager.wixmp.com/", "").split(".urlset").next().unwrap_or("").to_string();
            let re = Regex::new(r".*/,[^/],/mp4.*").unwrap();

            let mut output: Vec<(i32, String)> = Vec::new();

            if let Some(caps) = re.captures(x){
                let csv_group = &caps[1];
                for j in csv_group.split(','){
                    if j.is_empty(){continue;}
                    let formatted_line = format!("{} > {}", j , extracted_link).replace(",[^/]*", j);
                    let numeric_key:i32 = j.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse()?;
                    output.push((numeric_key, formatted_line));
                }

            }
            output.sort_by(|a,b| b.0.cmp(&a.0));
            Ok(output)
        },
        x if x.contains("master.m3u8")=>{
            let re = Regex::new(r#"Referer":"([^"]*)""#)?;
            let m3u8_refr = re.captures(response).map(|caps| caps[1].to_string()).unwrap_or_default();
            println!("{x}");
            let first_line = x.lines().next().unwrap_or("");
            let extract_link = first_line.split('>').nth(1).unwrap_or("");
            let relative_link = match extract_link.rfind('/'){
                Some(idx) => {
                    &x[..idx+1]
                },
                None=> "",
            };

            let payload = String::new();
            let handler = PostHandler{
                upload_data: payload.clone(),
                response_data: Vec::new()
            };
            let mut handle = Easy2::new(handler);
            handle.referer(&m3u8_refr)?;

            let mut headers = List::new();
            headers.append("Content-Type: application/json")?;
            handle.http_headers(headers)?;
            
            //handle.post(true)?;
            //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;
            
            
            handle.url(&extract_link)?;

            handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;
            

            let contents = handle.get_ref();
            let response =&std::str::from_utf8(&contents.response_data)?;
            println!("{response}");

            Ok(Vec::new())
        },
        x => {
            let link_vec = vec![(0, x.to_string())];
            Ok(link_vec)
        }
    }
}


fn get_episode_link(source_id: &str) -> Result<Vec<(i32, String)>, Box<dyn std::error::Error>>{
    //todo!("Finish grabbing links for episodes");
    
    let ref_url = "https://youtu-chan.com";
    let api_base =  "allanime.day";
    match source_id.to_lowercase(){
        x if x.contains("mp4upload")=>{
            println!("{x}");
            let payload = String::new();
            let handler = PostHandler{
                upload_data: payload.clone(),
                response_data: Vec::new()
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

            handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;
            

            let contents = handle.get_ref();
            let response =&std::str::from_utf8(&contents.response_data)?;
            //println!("{response}");
            let source_re = regex::Regex::new(r#".*src: "([^"]*)"\s*"#)?;
            if let Some(caps) = source_re.captures(*response){
                let links = extract_link(&caps[1], "")?;
                println!("{links:?}");
                Ok(links)
            } else {
                Err("Unable to capture source link".into())
            }


            //Ok(String::new())
        },
        x if x.contains("tools.fast4speed.rsvp") =>{
            let episode_link = vec![(0, x)];
            Ok(episode_link)

        },
        x => {
            let payload = String::new();
            let handler = PostHandler{
                upload_data: payload.clone(),
                response_data: Vec::new()
            };
            let mut handle = Easy2::new(handler);
            handle.referer(ref_url)?;

            let mut headers = List::new();
            headers.append("Content-Type: application/json")?;
            handle.http_headers(headers)?;
            
            //handle.post(true)?;
            //handle.post_field_size(payload.clone().as_bytes().len() as u64)?;
            
            
            handle.url(&format!("https://{api_base}{x}"))?;

            handle.useragent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0")?;

            #[cfg(debug_assertions)]
            handle.verbose(true)?;

            handle.perform()?;
            

            let contents = handle.get_ref();
            let response =&std::str::from_utf8(&contents.response_data)?;
            let root: episode_links::Root = serde_json::from_str(response)?;


            let links = extract_link(&root.links[0].link, response)?;
            
            //Ok(response.clone())
            Ok(links)
        }
    }
}


fn source_init(source_name: &str, source: &SourceUrl)->String{
    if let Some(source_url) = &source.source_url{
        if !source_url.starts_with("--"){
            return source_url.clone()
        } 

        let payload = &source_url[2..];
        let mut decoded = String::with_capacity(payload.len()/2);

        let mut chars = payload.chars();
        while let (Some(c1), Some(c2)) = (chars.next(), chars.next()){
            let pair: String = format!("{c1}{c2}");

            let mapped_char = match pair.as_str(){
                "79" => 'A', "7a" => 'B', "7b" => 'C', "7c" => 'D', "7d" => 'E', "7e" => 'F', "7f" => 'G',
                "70" => 'H', "71" => 'I', "72" => 'J', "73" => 'K', "74" => 'L', "75" => 'M', "76" => 'N',
                "77" => 'O', "68" => 'P', "69" => 'Q', "6a" => 'R', "6b" => 'S', "6c" => 'T', "6d" => 'U',
                "6e" => 'V', "6f" => 'W', "60" => 'X', "61" => 'Y', "62" => 'Z',
                "59" => 'a', "5a" => 'b', "5b" => 'c', "5c" => 'd', "5d" => 'e', "5e" => 'f', "5f" => 'g',
                "50" => 'h', "51" => 'i', "52" => 'j', "53" => 'k', "54" => 'l', "55" => 'm', "56" => 'n',
                "57" => 'o', "48" => 'p', "49" => 'q', "4a" => 'r', "4b" => 's', "4c" => 't', "4d" => 'u',
                "4e" => 'v', "4f" => 'w', "40" => 'x', "41" => 'y', "42" => 'z',
                "08" => '0', "09" => '1', "0a" => '2', "0b" => '3', "0c" => '4', "0d" => '5', "0e" => '6',
                "0f" => '7', "00" => '8', "01" => '9',
                "15" => '-', "16" => '.', "67" => '_', "46" => '~', "02" => ':', "17" => '/', "07" => '?',
                "1b" => '#', "63" => '[', "65" => ']', "78" => '@', "19" => '!', "1c" => '$', "1e" => '&',
                "10" => '(', "11" => ')', "12" => '*', "13" => '+', "14" => ',', "03" => ';', "05" => '=',
                "1d" => '%',
                _ => ' ',
            };
            decoded.push(mapped_char);
        }

        decoded.replace("/clock", "/clock.json")
    }else {
        return String::from("Unable to Parse Url")
    }
    
}


pub fn generate_link(source: &SourceUrl) -> Result<Vec<(i32, String)>, Box<dyn std::error::Error>>{
    if let Some(source_name) = &source.source_name{
        match source_name.as_str(){
            "Mp4" =>{
                let source_id = source_init("mp4upload", source);
                let episode_link = get_episode_link(&source_id)?;

                dbg!(&episode_link);
                Ok(episode_link)
            },
            "Fm-Hls" => {
                let source_id = source_init("Filemoon", source);
                Ok(Vec::new())
                },
            "Yt-mp4" => {
                let source_id = source_init("youtube", source);
                let episode_link = get_episode_link(&source_id)?;

                dbg!(&episode_link);
                Ok(episode_link)
                },
            "S-mp4" => {
                let source_id = source_init("sharepoint", source);
                let episode_link = get_episode_link(&source_id)?;
                
                dbg!(&episode_link);
                Ok(episode_link)
                },
                _ => {Ok(Vec::new())}      
        }
    } else {
        Err("Unable to get Source Name".into())
    }

}



fn encrypt_key(key: &str) -> Vec<u8>{
    use sha2::{Sha256, Digest};
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

fn process_response(url_data: &episodes::URLData, key: &str)-> Result<episode_source::Root,Box<dyn std::error::Error>>{
    use aes::cipher::{KeyIvInit, StreamCipher};
    type Aes256Ctr64Be = ctr::Ctr64BE<aes::Aes256>;

    let to_be_parsed = &url_data.tobeparsed;
    //println!("{to_be_parsed}");
    let bytes = base64::prelude::BASE64_STANDARD.decode(to_be_parsed)?;
    
    let buffer = &bytes[1..13];
    let mut ctr_block = [0u8;16];
    ctr_block[..12].copy_from_slice(buffer);
    ctr_block[12..16].copy_from_slice(&[0x00,0x00,0x00,0x02]);
    
    
    let ct_len = bytes.len()-16;

    let mut encrypted_buffer = bytes[13..(ct_len)].to_vec();

    let decoded_key = hex::decode(key)?;
    let mut decryptor = Aes256Ctr64Be::new_from_slices(&decoded_key, &ctr_block)?;
    decryptor.apply_keystream(&mut encrypted_buffer);

    let response = String::from_utf8(encrypted_buffer.to_vec())?;

    dbg!(&response);
    let response_json: episode_source::Root= serde_json::from_str(&response)?;
    Ok(response_json)
}

fn get_aa_req(aa_key: &str, epoch: i32, build_id: i32, query_hash: &str)->Result<String, Box<dyn std::error::Error>>{
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce, Key};
    
    use hybrid_array::{Array, sizes};
    use chrono::Utc;
    let ts = (Utc::now().timestamp()/300)*300*1000;
    println!("{ts}");

    let decoded_key = hex::decode(aa_key)?;

    let payload_iv = format!("{epoch}:{build_id}:{query_hash}:{ts}");
    let payload = json!({
        "v": 1,
        "ts": ts,
        "epoch": epoch,
        "buildId": build_id,
        "qh": query_hash
    }).to_string();


    let encrypted_iv= encrypt_key(&payload_iv);
    let encrypted_bytes = &encrypted_iv[..12];

    // let key: &Array<u8, sizes::U32> = &Key::<Aes256Gcm>::try_from(&decoded_key as &[u8])?;
    let cipher = Aes256Gcm::new_from_slice(&decoded_key)?;
    let nonce:&Array<u8, sizes::U12> = &Nonce::try_from(encrypted_bytes)?;

    let cipher_text = cipher.encrypt(nonce, payload.as_bytes())?;

    let mut buffer = Vec::with_capacity(1+12+cipher_text.len());
    buffer.push(0x01);
    buffer.extend_from_slice(encrypted_bytes);
    buffer.extend_from_slice(&cipher_text);
    


    let b64_string = base64::prelude::BASE64_STANDARD.encode(&buffer);
    
    Ok(b64_string)

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

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;
    

    let contents = handle.get_ref();
    let response =&std::str::from_utf8(&contents.response_data)?.replace("_", "");
    Ok(response.clone())
}



pub fn get_shows(show: &String, translation_type: &Translation) -> Result<Vec<Edge>, Box<dyn std::error::Error>>{

    let search_gql="query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}";
    

    let payload = json!({
        "variables" :{
            "search": {
                "allowAdult" : true,
                "allowUnknown" : false,
                "query" : show
            },
            "limit" : 40,
            "page" : 1,
            "translationType" : match translation_type {
                Translation::Sub => "sub",
                Translation::Dub => "dub",
                Translation::Raw => "raw"
            },
            "countryOrigin" : "ALL"
        },
        "query" : search_gql
    }).to_string();
    
    dbg!(&payload);

    //format!("{{\"variables\":{{\"search\":{{\"allowAdult\":true,\"allowUnknown\":false,\"query\":\"{show}\"}},\"limit\":40,\"page\":1,\"translationType\":\"{translation_type}\",\"countryOrigin\":\"ALL\"}},\"query\":\"{search_gql}\"}}");

    let response = post_curl(payload)?;
    // let json: Value = serde_json::from_str(&response)?; 
    // println!("Response Body: {json:?}");

    let data:shows::Root = serde_json::from_str(&response)?;
    dbg!(&data);

    
    Ok(data.data.shows.edges)
}

pub fn get_episode_list(show_id: &String) -> Result<episodes::AvailableEpisodesDetail, Box<dyn std::error::Error>>{
    let episode_list_gql = "query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}";

    let payload = format!("{{\"variables\":{{\"showId\":\"{show_id}\"}},\"query\":\"{episode_list_gql}\"}}");

    let response = post_curl(payload)?;
    // let json:Value = serde_json::from_str(&response)?;
    // println!("{:?}", json);

    let data:episodes::Root = serde_json::from_str(&response)?;
    dbg!(&data);


    Ok(data.data.show.available_episodes_detail)

}

pub fn get_episode_url(show_id: &String, translation_type: &String,  episode_num: &String)->Result<Vec<(String,Vec<(i32,String)>)>, Box<dyn std::error::Error>>{

    let episode_embed_gql="query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}";
    
    let epoch = 4130;
    let build_id = 41;
    let key = "cf4777b5778aeadc9449e12769ea545d00c43cd8ff65d482364586cde204f359";
    let query_hash = "d405d0edd690624b66baba3068e0edc3ac90f1597d898a1ec8db4e5c43c00fec";
    

    //{\"persistedQuery\":{\"version\":1,\"sha256Hash\":\"$allanime_query_hash\"}, \"aaReq\":\"$(get_aa_req)\"}
    let aa_req = get_aa_req(key, epoch, build_id, query_hash)?;
    //let aa_req = "Ab2YhqeMcQIxhA60YBrlJAPsZz8ar1ekpoZWbwjwuWdH2mfeYnGG4gx1WsLTxHAzhULq9ddLXYNI9RLQdfI/lfoIvBSvEsUq5BsvLNfF3u6PcOUwhouC51CrrH00BHYR+4KHz4t56ZDsJFThDEFok5RiyHK2N2HImm3XglhXZ7nfmgT08m4mzt9AZ20mC4ueZGnffF7y3LE5/7o=";
    dbg!(&aa_req);
    let query_vars = json!({
        "showId": show_id,
        "translationType": translation_type,
        "episodeString": episode_num
    }).to_string();
    //let query_vars=format!("{{\"showId\":\"{show_id}\",\"translationType\":\"{translation_type}\",\"episodeString\":\"{episode_num}\"}}");
    let query_ext = json!({
        "persistedQuery" : {
            "version": 1,
            "sha256Hash": query_hash
        },
        "aaReq": aa_req
    }).to_string();
    //let query_ext=format!("{{\"persistedQuery\":{{\"version\":1,\"sha256Hash\":\"{query_hash}\"}}, \"aaReq\":\"{aa_req}\" }}");
    
    let payload = format!("variables={}&extensions={}", encode(&query_vars), encode(&query_ext));
    dbg!(&payload);

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
    let response =std::str::from_utf8(&contents.response_data)?;
    dbg!(&response);
    
    let root: episodes::URLRoot = serde_json::from_str(response)?;
    // if root.data.tobeparsed.is_empty(){
    //     let payload = format!("{{\"variables\":{query_vars},\"query\":\"{episode_embed_gql}\" }}");
    //     let response = post_curl(payload)?;
    //     let
    // } else {
    let response = process_response(&root.data, key)?;
    if let Some(episode) = response.episode{
        if let Some(source_url) = episode.source_urls{
            let mut urls:Vec<(String,Vec<(i32,String)>)> = Vec::new();
            for url in source_url{
                let source_name = match &url.source_name{
                    Some(source) => source,
                    None => "No Source available"
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
