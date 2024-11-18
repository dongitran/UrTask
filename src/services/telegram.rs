use crate::error::AppError;
use reqwest::Client;
use serde_json::json;

pub async fn send_message(bot_token: &str, chat_id: i64, message: &str) -> Result<(), AppError> {
    println!("Attempting to send message to chat_id: {}", chat_id);
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let client = Client::new();

    let response = client
        .post(&url)
        .json(
            &json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "Markdown"
        })
        )
        .send().await?;

    if response.status().is_success() {
        println!("Message sent successfully to chat_id: {}", chat_id);
        Ok(())
    } else {
        let error_text = response.text().await?;
        println!("Failed to send message to chat_id: {}. Error: {}", chat_id, error_text);
        Err(AppError::from(format!("Failed to send message: {}", error_text)))
    }
}
