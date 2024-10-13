use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrelloConfig {
    pub board: String,
    pub key: String,
    pub token: String,
    #[serde(rename = "userId")]
    pub user_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct TrelloList {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct TrelloCard {
    pub id: String,
    pub name: String,
    #[serde(rename = "idList")]
    pub id_list: String,
}