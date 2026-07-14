use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub show: Show,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub available_episodes_detail: AvailableEpisodesDetail,
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableEpisodesDetail {
    pub dub: Vec<Option<String>>,
    pub raw: Vec<Option<String>>,
    pub sub: Vec<Option<String>>,
}