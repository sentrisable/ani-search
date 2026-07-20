use curl::easy::{Easy2, List};
use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::{Config, PostHandler, anilist_shows};


#[derive(Debug, Default, Serialize, Deserialize,PartialEq)]
pub struct AnilistVariables{
    pub token: String,
    pub user_id: i64,
}




pub fn get_anime_from_list(user_id: i64, token: &str)->Result<anilist_shows::MediaListCollection, Box<dyn std::error::Error>>{    // $sed "s@},{@\n@g" | $sed -nE "s@.*\"mediaId\":([0-9]*),\"status\":\"($1)\",\"score\":(.*),\"progress\":([0-9]*),.*\"userPreferred\":\"([^\"]*)\".*\"coverImage\":\{\"extraLarge\":\"([^\"]*)\".*\"episode([\"]*)s*[\"]*:([0-9]*).*\"startDate\":\{\"year\":([0-9]*).*@\6\t\1\t\5 \(\9\) \4|\8 episodes \7 \[\3\]@p"
    let payload = json!({
        "query" : "query( $userId:Int, $userName:String, $type:MediaType){MediaListCollection(userId:$userId,userName:$userName,type:$type){lists{name isCustomList isCompletedList:isSplitCompletedList entries{...mediaListEntry}}user{id name avatar{large}mediaListOptions{scoreFormat rowOrder animeList{sectionOrder customLists splitCompletedSectionByFormat theme}mangaList{sectionOrder customLists splitCompletedSectionByFormat theme}}}}}fragment mediaListEntry on MediaList{id mediaId status score progress progressVolumes repeat priority private hiddenFromStatusLists customLists advancedScores notes updatedAt startedAt{year month day}completedAt{year month day}media{id title{userPreferred romaji english native}coverImage{extraLarge large}type format status(version:2)episodes volumes chapters averageScore popularity isAdult countryOfOrigin genres bannerImage nextAiringEpisode{airingAt timeUntilAiring episode} startDate{year month day}}}",
        "variables": {
            "userId" : user_id,
            "type" : "ANIME"
        }
    }).to_string();

    let handler = PostHandler{
        upload_data: payload.clone(),
        response_data: Vec::new()
    };

    let mut handle = Easy2::new(handler);
    let mut headers = List::new();
    headers.append("Content-Type: application/json")?;
    headers.append(format!("Authorization: Bearer {token}").as_str())?;
    handle.http_headers(headers)?;

    handle.post(true)?;
    handle.post_field_size(payload.as_bytes().len() as u64)?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.url("https://graphql.anilist.co")?;
    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;
    
    let root: anilist_shows::Root = serde_json::from_str(*response)?;

    Ok(root.data.media_list_collection)
}


pub fn search_anilist(show: &String, config: &Config) -> Result<(), Box<dyn std::error::Error>>{
    let is_adult = &config.app_settings.allow_adult;
    let token = &config.anilist.token;
    //"{\"query\":\"query($page:Int = 1 $id:Int $type:MediaType $isAdult:Boolean = false $search:String $format:[MediaFormat]$status:MediaStatus $countryOfOrigin:CountryCode $source:MediaSource $season:MediaSeason $seasonYear:Int $year:String $onList:Boolean $yearLesser:FuzzyDateInt $yearGreater:FuzzyDateInt $episodeLesser:Int $episodeGreater:Int $durationLesser:Int $durationGreater:Int $chapterLesser:Int $chapterGreater:Int $volumeLesser:Int $volumeGreater:Int $licensedBy:[Int]$isLicensed:Boolean $genres:[String]$excludedGenres:[String]$tags:[String]$excludedTags:[String]$minimumTagRank:Int $sort:[MediaSort]=[POPULARITY_DESC,SCORE_DESC]){Page(page:$page,perPage:20){pageInfo{total perPage currentPage lastPage hasNextPage}media(id:$id type:$type season:$season format_in:$format status:$status countryOfOrigin:$countryOfOrigin source:$source search:$search onList:$onList seasonYear:$seasonYear startDate_like:$year startDate_lesser:$yearLesser startDate_greater:$yearGreater episodes_lesser:$episodeLesser episodes_greater:$episodeGreater duration_lesser:$durationLesser duration_greater:$durationGreater chapters_lesser:$chapterLesser chapters_greater:$chapterGreater volumes_lesser:$volumeLesser volumes_greater:$volumeGreater licensedById_in:$licensedBy isLicensed:$isLicensed genre_in:$genres genre_not_in:$excludedGenres tag_in:$tags tag_not_in:$excludedTags minimumTagRank:$minimumTagRank sort:$sort isAdult:$isAdult){id title{userPreferred}coverImage{extraLarge large color}startDate{year month day}endDate{year month day}bannerImage season seasonYear description type format status(version:2)episodes duration chapters volumes genres isAdult averageScore popularity nextAiringEpisode{airingAt timeUntilAiring episode}mediaListEntry{id status}studios(isMain:true){edges{isMain node{id name}}}}}}\",\"variables\":{\"page\":1,\"type\":\"ANIME\",\"sort\":\"SEARCH_MATCH\",\"search\":\"$1\"}}" 
    let payload = json!({
        "query" : format!("query ($page:Int = 1 $id:Int $type:MediaType $isAdult:Boolean = false $search:String $format:[MediaFormat]$status:MediaStatus $countryOfOrigin:CountryCode $source:MediaSource $season:MediaSeason $seasonYear:Int $year:String $onList:Boolean $yearLesser:FuzzyDateInt $yearGreater:FuzzyDateInt $episodeLesser:Int $episodeGreater:Int $durationLesser:Int $durationGreater:Int $chapterLesser:Int $chapterGreater:Int $volumeLesser:Int $volumeGreater:Int $licensedBy:[Int]$isLicensed:Boolean $genres:[String]$excludedGenres:[String]$tags:[String]$excludedTags:[String]$minimumTagRank:Int $sort:[MediaSort]=[POPULARITY_DESC,SCORE_DESC]){{Page(page:$page,perPage:20){{pageInfo{{total perPage currentPage lastPage hasNextPage}}media(id:$id type:$type season:$season format_in:$format status:$status countryOfOrigin:$countryOfOrigin source:$source search:$search onList:$onList seasonYear:$seasonYear startDate_like:$year startDate_lesser:$yearLesser startDate_greater:$yearGreater episodes_lesser:$episodeLesser episodes_greater:$episodeGreater duration_lesser:$durationLesser duration_greater:$durationGreater chapters_lesser:$chapterLesser chapters_greater:$chapterGreater volumes_lesser:$volumeLesser volumes_greater:$volumeGreater licensedById_in:$licensedBy isLicensed:$isLicensed genre_in:$genres genre_not_in:$excludedGenres tag_in:$tags tag_not_in:$excludedTags minimumTagRank:$minimumTagRank sort:$sort isAdult:{is_adult}){{id title{{userPreferred}}coverImage{{extraLarge large color}}startDate{{year month day}}endDate{{year month day}}bannerImage season seasonYear description type format status(version:2)episodes duration chapters volumes genres isAdult averageScore popularity nextAiringEpisode{{airingAt timeUntilAiring episode}}mediaListEntry{{id status}}studios(isMain:true){{edges{{isMain node{{id name}}}}}}}}}}}}\""),
        "variables":{
            "page":1,
            "type":"ANIME",
            "sort":"SEARCH_MATCH",
            "search":show}
    }).to_string();
        let handler = PostHandler{
        upload_data: payload.clone(),
        response_data: Vec::new()
    };

    let mut handle = Easy2::new(handler);
    let mut headers = List::new();
    headers.append(format!("Authorization: Bearer {token}").as_str())?;
    headers.append("Content-Type: application/json")?;
    headers.append("Accept: application/json")?;
    
    handle.http_headers(headers)?;

    handle.post(true)?;
    handle.post_field_size(payload.as_bytes().len() as u64)?;

    handle.url("https://graphql.anilist.co")?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;
    dbg!(response);
    Ok(())
}


pub fn check_credentials(token: &str)->Result<i64, Box<dyn std::error::Error>>{
    let payload = json!({
        "query" : "query { Viewer { id } }"
    }).to_string();


    let handler = PostHandler{
        upload_data: payload.clone(),
        response_data: Vec::new()
    };

    let mut handle = Easy2::new(handler);
    let mut headers = List::new();
    headers.append(format!("Authorization: Bearer {token}").as_str())?;
    headers.append("Content-Type: application/json")?;
    headers.append("Accept: application/json")?;
    
    handle.http_headers(headers)?;

    handle.post(true)?;
    handle.post_field_size(payload.as_bytes().len() as u64)?;

    handle.url("https://graphql.anilist.co")?;

    #[cfg(debug_assertions)]
    handle.verbose(true)?;

    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;

    let root:serde_json::Value = serde_json::from_str(*response)?;
    let id = &root["data"]["Viewer"]["id"];
    if id.is_number() == true{
        Ok(id.as_i64().unwrap())
    } else{
        Err("Unable to parse ID".into())
    }

}