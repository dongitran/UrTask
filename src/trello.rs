use reqwest::Client;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Debug)]
pub struct TrelloList {
    pub id: String,
    pub name: String,
    pub closed: bool,
    pub idBoard: String,
    pub pos: f64,
}

#[derive(Deserialize, Debug)]
pub struct TrelloCard {
    pub id: String,
    pub name: String,
    pub idList: String,
}

#[derive(Debug)]
pub struct TrelloCheckResult {
    pub status: bool,
    pub todo_id: Option<String>,
    pub doing_id: Option<String>,
    pub done_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrelloConfig {
    pub board: String,
    pub key: String,
    pub token: String,
    #[serde(rename = "userId")]
    pub user_id: i64,
}

pub async fn check_trello_lists(
    key: &str,
    token: &str,
    board: &str
) -> Result<TrelloCheckResult, String> {
    let client = Client::new();
    let url = format!(
        "https://api.trello.com/1/boards/{}/lists?key={}&token={}",
        board,
        key,
        token
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await
        .map_err(|_| "Lỗi khi gửi yêu cầu.".to_string())?;

    if response.status().is_success() {
        let lists: Vec<TrelloList> = response
            .json().await
            .map_err(|_| "Lỗi khi parse dữ liệu.".to_string())?;

        let mut result = TrelloCheckResult {
            status: false,
            todo_id: None,
            doing_id: None,
            done_id: None,
        };

        for list in &lists {
            match list.name.as_str() {
                "ToDo" => result.todo_id = Some(list.id.clone()),
                "Doing" => result.doing_id = Some(list.id.clone()),
                "Done" => result.done_id = Some(list.id.clone()),
                _ => {}
            }
        }

        if result.todo_id.is_some() && result.doing_id.is_some() && result.done_id.is_some() {
            result.status = true;
            Ok(result)
        } else {
            Err("Danh sách KHÔNG có đủ 3 tên 'ToDo', 'Doing', và 'Done'.".to_string())
        }
    } else {
        Err(format!("Lỗi: {}", response.status()))
    }
}

pub async fn get_trello_cards(key: &str, token: &str, board: &str) -> Result<Vec<TrelloCard>, String> {
    let client = Client::new();
    let url = format!(
        "https://api.trello.com/1/boards/{}/cards?key={}&token={}",
        board,
        key,
        token
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await
        .map_err(|_| "Lỗi khi gửi yêu cầu lấy thẻ.".to_string())?;

    if response.status().is_success() {
        let cards: Vec<TrelloCard> = response
            .json().await
            .map_err(|_| "Lỗi khi parse dữ liệu thẻ.".to_string())?;
        Ok(cards)
    } else {
        Err(format!("Lỗi: {}", response.status()))
    }
}

pub fn map_cards_to_lists(
    result: TrelloCheckResult,
    cards: Vec<TrelloCard>
) -> (Vec<TrelloCard>, Vec<TrelloCard>, Vec<TrelloCard>) {
    let mut todo_cards = Vec::new();
    let mut doing_cards = Vec::new();
    let mut done_cards = Vec::new();

    for card in cards {
        match card.idList.as_str() {
            list_id if Some(list_id) == result.todo_id.as_deref() => todo_cards.push(card),
            list_id if Some(list_id) == result.doing_id.as_deref() => doing_cards.push(card),
            list_id if Some(list_id) == result.done_id.as_deref() => done_cards.push(card),
            _ => {}
        }
    }

    (todo_cards, doing_cards, done_cards)
}