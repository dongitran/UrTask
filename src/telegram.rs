use reqwest::Client;
use serde_json::json;

pub async fn send_message(
    bot_token: &str,
    chat_id: i64,
    message: &str
) -> Result<(), Box<dyn std::error::Error>> {
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
    } else {
        println!("Failed to send message: {:?}", response.text().await?);
    }

    Ok(())
}