use crate::error::AppError;
use crate::models::trello::{ TrelloCard, TrelloList, TrelloConfig };
use reqwest::Client;

pub async fn check_trello_lists(
    config: &TrelloConfig
) -> Result<(String, String, String), AppError> {
    let client = Client::new();
    let url = format!(
        "https://api.trello.com/1/boards/{}/lists?key={}&token={}",
        config.board,
        config.key,
        config.token
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(AppError::from(format!("Trello API error: Status {}, Body: {}", status, text)));
    }

    let lists: Vec<TrelloList> = response.json().await?;

    let mut todo_id = None;
    let mut doing_id = None;
    let mut done_id = None;

    for list in lists {
        match list.name.as_str() {
            "ToDo" => {
                todo_id = Some(list.id);
            }
            "Doing" => {
                doing_id = Some(list.id);
            }
            "Done" => {
                done_id = Some(list.id);
            }
            _ => {}
        }
    }

    match (todo_id, doing_id, done_id) {
        (Some(todo), Some(doing), Some(done)) => Ok((todo, doing, done)),
        _ =>
            Err(
                AppError::from(
                    "Không tìm thấy đủ 3 danh sách 'ToDo', 'Doing' và 'Done'".to_string()
                )
            ),
    }
}

pub async fn get_trello_cards(config: &TrelloConfig) -> Result<Vec<TrelloCard>, AppError> {
    let client = Client::new();
    let url = format!(
        "https://api.trello.com/1/boards/{}/cards?key={}&token={}",
        config.board,
        config.key,
        config.token
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(AppError::from(format!("Trello API error: Status {}, Body: {}", status, text)));
    }

    let response_text = response.text().await?;
    //println!("Trello API response for cards: {}", response_text);

    let cards: Vec<TrelloCard> = serde_json
        ::from_str(&response_text)
        .map_err(|e| AppError::from(format!("Failed to parse Trello cards: {}", e)))?;

    Ok(cards)
}

pub fn map_cards_to_lists(
    cards: Vec<TrelloCard>,
    todo_id: &str,
    doing_id: &str,
    done_id: &str
) -> (Vec<TrelloCard>, Vec<TrelloCard>, Vec<TrelloCard>) {
    let mut todo_cards = Vec::new();
    let mut doing_cards = Vec::new();
    let mut done_cards = Vec::new();

    for card in cards {
        match card.id_list.as_str() {
            id if id == todo_id => todo_cards.push(card),
            id if id == doing_id => doing_cards.push(card),
            id if id == done_id => done_cards.push(card),
            _ => {}
        }
    }

    (todo_cards, doing_cards, done_cards)
}
