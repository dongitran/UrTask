use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub async fn send_message(
    bot_token: &str,
    chat_id: i64,
    message: &str
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let client = Client::new();

    let response = client
        .post(&url)
        .json(&json!({
            "chat_id": chat_id,
            "text": message,
        }))
        .send().await?;

    if response.status().is_success() {
        println!("Message sent successfully!");
        Ok(())
    } else {
        let error_text = response.text().await?;
        Err(format!("Failed to send message: {}", error_text).into())
    }
}