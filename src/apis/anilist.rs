use crate::{Config, PostHandler, anilist_search_shows, anilist_user_shows, output_json};
use curl::easy::{Easy2, List};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AnilistVariables {
    pub token: String,
    pub user_id: i64,
}

#[derive(Debug, PartialEq)]
pub enum WatchStatus {
    Watching,
    Repeating,
    Completed,
    Paused,
    Dropped,
    Planning,
    None,
}

fn post_anilist(
    payload: &String,
    token: &Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let handler = PostHandler {
        upload_data: payload.clone(),
        response_data: Vec::new(),
    };

    let mut handle = Easy2::new(handler);
    let mut headers = List::new();
    headers.append("Content-Type: application/json")?;
    headers.append("Accept: application/json")?;
    match token {
        Some(t) => headers.append(format!("Authorization: Bearer {t}").as_str())?,
        None => (),
    }
    handle.http_headers(headers)?;

    handle.post(true)?;
    handle.post_field_size(payload.as_bytes().len() as u64)?;

    //#[cfg(debug_assertions)]
    //handle.verbose(true)?;

    handle.url("https://graphql.anilist.co")?;
    handle.perform()?;

    let contents = handle.get_ref();
    let response = &std::str::from_utf8(&contents.response_data)?;

    Ok(response.to_string())
}

pub fn get_anime_from_list(
    user_id: i64,
    token: &Option<&str>,
) -> Result<anilist_user_shows::MediaListCollection, Box<dyn std::error::Error>> {
    let payload = json!({
        "query" : "query( $userId:Int, $userName:String, $type:MediaType){MediaListCollection(userId:$userId,userName:$userName,type:$type){lists{name isCustomList isCompletedList:isSplitCompletedList entries{...mediaListEntry}}user{id name avatar{large}mediaListOptions{scoreFormat rowOrder animeList{sectionOrder customLists splitCompletedSectionByFormat theme}mangaList{sectionOrder customLists splitCompletedSectionByFormat theme}}}}}fragment mediaListEntry on MediaList{id mediaId status score progress progressVolumes repeat priority private hiddenFromStatusLists customLists advancedScores notes updatedAt startedAt{year month day}completedAt{year month day}media{id title{userPreferred romaji english native}coverImage{extraLarge large}type format status(version:2)episodes volumes chapters averageScore popularity isAdult countryOfOrigin genres bannerImage nextAiringEpisode{airingAt timeUntilAiring episode} startDate{year month day}}}",
        "variables": {
            "userId" : user_id,
            "type" : "ANIME"
        }
    }).to_string();

    let response = post_anilist(&payload, token)?;

    let root: anilist_user_shows::Root = serde_json::from_str(&response)?;

    Ok(root.data.media_list_collection)
}

pub fn search_anilist(
    show: &String,
    token: &Option<&str>,
) -> Result<anilist_search_shows::Page, Box<dyn std::error::Error>> {
    //"{\"query\":\"query($page:Int = 1 $id:Int $type:MediaType $isAdult:Boolean = false $search:String $format:[MediaFormat]$status:MediaStatus $countryOfOrigin:CountryCode $source:MediaSource $season:MediaSeason $seasonYear:Int $year:String $onList:Boolean $yearLesser:FuzzyDateInt $yearGreater:FuzzyDateInt $episodeLesser:Int $episodeGreater:Int $durationLesser:Int $durationGreater:Int $chapterLesser:Int $chapterGreater:Int $volumeLesser:Int $volumeGreater:Int $licensedBy:[Int]$isLicensed:Boolean $genres:[String]$excludedGenres:[String]$tags:[String]$excludedTags:[String]$minimumTagRank:Int $sort:[MediaSort]=[POPULARITY_DESC,SCORE_DESC]){Page(page:$page,perPage:20){pageInfo{total perPage currentPage lastPage hasNextPage}media(id:$id type:$type season:$season format_in:$format status:$status countryOfOrigin:$countryOfOrigin source:$source search:$search onList:$onList seasonYear:$seasonYear startDate_like:$year startDate_lesser:$yearLesser startDate_greater:$yearGreater episodes_lesser:$episodeLesser episodes_greater:$episodeGreater duration_lesser:$durationLesser duration_greater:$durationGreater chapters_lesser:$chapterLesser chapters_greater:$chapterGreater volumes_lesser:$volumeLesser volumes_greater:$volumeGreater licensedById_in:$licensedBy isLicensed:$isLicensed genre_in:$genres genre_not_in:$excludedGenres tag_in:$tags tag_not_in:$excludedTags minimumTagRank:$minimumTagRank sort:$sort isAdult:$isAdult){id title{userPreferred}coverImage{extraLarge large color}startDate{year month day}endDate{year month day}bannerImage season seasonYear description type format status(version:2)episodes duration chapters volumes genres isAdult averageScore popularity nextAiringEpisode{airingAt timeUntilAiring episode}mediaListEntry{id status}studios(isMain:true){edges{isMain node{id name}}}}}}\",\"variables\":{\"page\":1,\"type\":\"ANIME\",\"sort\":\"SEARCH_MATCH\",\"search\":\"$1\"}}"
    let payload = json!({
        "query" : format!("query ($page:Int = 1 $id:Int $type:MediaType $search:String $format:[MediaFormat]$status:MediaStatus $countryOfOrigin:CountryCode $source:MediaSource $season:MediaSeason $seasonYear:Int $year:String $onList:Boolean $yearLesser:FuzzyDateInt $yearGreater:FuzzyDateInt $episodeLesser:Int $episodeGreater:Int $durationLesser:Int $durationGreater:Int $chapterLesser:Int $chapterGreater:Int $volumeLesser:Int $volumeGreater:Int $licensedBy:[Int]$isLicensed:Boolean $genres:[String]$excludedGenres:[String]$tags:[String]$excludedTags:[String]$minimumTagRank:Int $sort:[MediaSort]=[POPULARITY_DESC,SCORE_DESC]){{Page(page:$page,perPage:20){{pageInfo{{total perPage currentPage lastPage hasNextPage}}media(id:$id type:$type season:$season format_in:$format status:$status countryOfOrigin:$countryOfOrigin source:$source search:$search onList:$onList seasonYear:$seasonYear startDate_like:$year startDate_lesser:$yearLesser startDate_greater:$yearGreater episodes_lesser:$episodeLesser episodes_greater:$episodeGreater duration_lesser:$durationLesser duration_greater:$durationGreater chapters_lesser:$chapterLesser chapters_greater:$chapterGreater volumes_lesser:$volumeLesser volumes_greater:$volumeGreater licensedById_in:$licensedBy isLicensed:$isLicensed genre_in:$genres genre_not_in:$excludedGenres tag_in:$tags tag_not_in:$excludedTags minimumTagRank:$minimumTagRank sort:$sort ){{id title{{userPreferred romaji english}}coverImage{{extraLarge large color}}startDate{{year month day}}endDate{{year month day}}bannerImage season seasonYear description type format status(version:2)episodes duration chapters volumes genres averageScore popularity nextAiringEpisode{{airingAt timeUntilAiring episode}}mediaListEntry{{id status}}studios(isMain:true){{edges{{isMain node{{id name}}}}}}}}}}}}"),
        "variables":{
            "page":1,
            "type":"ANIME",
            "sort":"SEARCH_MATCH",
            "search":show,
        }
    }).to_string();
    let response = post_anilist(&payload, token)?;
    let root: anilist_search_shows::Root = serde_json::from_str(&response)?;
    let search_pages = root.data.page;
    Ok(search_pages)
}

pub fn check_credentials(token: &Option<&str>) -> Result<i64, Box<dyn std::error::Error>> {
    let payload = json!({
        "query" : "query { Viewer { id } }"
    })
    .to_string();

    let response = post_anilist(&payload, token)?;
    let root: serde_json::Value = serde_json::from_str(&response)?;
    let id = &root["data"]["Viewer"]["id"];
    if id.is_number() == true {
        Ok(id.as_i64().unwrap())
    } else {
        Err("Unable to parse ID".into())
    }
}

pub fn update_progress(
    show_id: &i64,
    episode_number: Option<&String>,
    status: WatchStatus,
    token: &Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    dbg!(&show_id);
    dbg!(&episode_number);
    let payload = json!({
        "query" : r#"mutation($id:Int $mediaId:Int $status:MediaListStatus $score:Float $progress:Int $progressVolumes:Int $repeat:Int $private:Boolean $notes:String $customLists:[String]$hiddenFromStatusLists:Boolean $advancedScores:[Float]$startedAt:FuzzyDateInput $completedAt:FuzzyDateInput){SaveMediaListEntry(id:$id mediaId:$mediaId status:$status score:$score progress:$progress progressVolumes:$progressVolumes repeat:$repeat private:$private notes:$notes customLists:$customLists hiddenFromStatusLists:$hiddenFromStatusLists advancedScores:$advancedScores startedAt:$startedAt completedAt:$completedAt){id mediaId status score advancedScores progress progressVolumes repeat priority private hiddenFromStatusLists customLists notes updatedAt startedAt{year month day}completedAt{year month day}user{id name}media{id title{userPreferred english romaji}coverImage{large}type format status episodes volumes chapters averageScore popularity isAdult startDate{year}}}}"#,
        "variables" : {
            "status" : match status{
                WatchStatus::None => "",
                WatchStatus::Completed => "COMPLETED",
                WatchStatus::Watching => "CURRENT",
                WatchStatus::Dropped => "DROPPED",
                WatchStatus::Paused => "PAUSED",
                WatchStatus::Planning => "PLANNING",
                WatchStatus::Repeating => "REPEATING",
            },
            "progress" : episode_number,
            "mediaId" : show_id
        }
    }).to_string();

    let response = post_anilist(&payload, token)?;
    dbg!(&response);
    Ok(())
}
