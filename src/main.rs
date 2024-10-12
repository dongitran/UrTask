use reqwest::Client;
use mongodb::{Client as MongoClient, options::ClientOptions};
use futures_util::stream::TryStreamExt;
use std::error::Error;
use serde_json::json;
use clokwerk::{Scheduler, TimeUnits};
use chrono::{FixedOffset, Utc, Datelike};
use std::sync::Arc;
use tokio::sync::Mutex;
use teloxide::{prelude::*, utils::command::BotCommands};
use serde::{Serialize, Deserialize};
use mongodb::options::UpdateOptions;
use mongodb::bson::{doc, Document};
use std::env;
use dotenv::dotenv;

mod trello;
mod telegram;
mod database;

use trello::{TrelloConfig, TrelloCheckResult, TrelloCard, check_trello_lists, get_trello_cards, map_cards_to_lists};
use telegram::send_message;
use database::{save_config_to_mongodb, save_reported_task, is_task_reported};

async fn process_user_config(config: &TrelloConfig, bot_token: &str, client: &MongoClient) -> Result<(), Box<dyn std::error::Error>> {
    match check_trello_lists(&config.key, &config.token, &config.board).await {
        Ok(result) => {
            let cards = get_trello_cards(&config.key, &config.token, &config.board).await?;
            let (todo_cards, doing_cards, done_cards) = map_cards_to_lists(result, cards);

            let mut new_done_cards = Vec::new();
            for card in &done_cards {
                if !is_task_reported(client, &card.id).await? {
                    new_done_cards.push(card);
                    save_reported_task(client, &card.id).await?;
                }
            }

            if !new_done_cards.is_empty() || !doing_cards.is_empty() || !todo_cards.is_empty() {
                let message = generate_report_message(&todo_cards, &doing_cards, &new_done_cards);
                send_message(bot_token, config.user_id, &message).await?;
            }
        }
        Err(err) => {
            println!("Error for user {}: {}", config.user_id, err);
            let error_message = "Lỗi: Không thể tìm thấy đủ 3 danh sách 'ToDo', 'Doing' và 'Done' trong bảng Trello của bạn. Vui lòng thiết lập các danh sách này để bot hoạt động chính xác.";
            send_message(bot_token, config.user_id, error_message).await?;
        }
    }
    Ok(())
}

fn generate_report_message(todo_cards: &[TrelloCard], doing_cards: &[TrelloCard], done_cards: &[&TrelloCard]) -> String {
    let today = Utc::now().naive_utc().date();
    let yesterday = today.pred_opt().expect("Failed to get previous day");

    let mut message = format!("Báo cáo ngày {}:\n\n", today);

    if !done_cards.is_empty() {
        message.push_str("Công việc đã hoàn thành:\n");
        for card in done_cards {
            message.push_str(&format!("- {}\n", card.name));
        }
        message.push_str("\n");
    }

    if !doing_cards.is_empty() {
        message.push_str("Công việc đang thực hiện:\n");
        for card in doing_cards {
            message.push_str(&format!("- {}\n", card.name));
        }
        message.push_str("\n");
    }

    if !todo_cards.is_empty() {
        message.push_str("Công việc cần làm:\n");
        for card in todo_cards {
            message.push_str(&format!("- {}\n", card.name));
        }
    }

    message
}

async fn cron_job() -> Result<(), Box<dyn std::error::Error>> {
    let connection_string = env::var("MONGODB_CONNECTION_STRING").expect("MONGODB_CONNECTION_STRING must be set");
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    let client_options = ClientOptions::parse(&connection_string).await?;
    let client = MongoClient::with_options(client_options)?;
    let database = client.database("urtask");
    let collection = database.collection::<TrelloConfig>("trello_configs");

    let mut cursor = collection.find(None, None).await?;

    while let Some(config) = cursor.try_next().await? {
        process_user_config(&config, &bot_token, &client).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let scheduler = Arc::new(Mutex::new(Scheduler::with_tz(FixedOffset::east_opt(7 * 3600).expect("Invalid offset"))));

    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let bot = Bot::new(&bot_token);

    let bot_handle = tokio::spawn(Command::repl(bot, answer));

    {
        let scheduler_clone = Arc::clone(&scheduler);
        tokio::spawn(async move {
            let mut scheduler = scheduler_clone.lock().await;

            scheduler.every(10.seconds()).run(move || {
                tokio::spawn(async move {
                    if let Err(e) = cron_job().await {
                        eprintln!("Error running cron job: {:?}", e);
                    }
                });
            });

            loop {
                scheduler.run_pending();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    if let Err(e) = bot_handle.await {
        eprintln!("Bot task error: {:?}", e);
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "set Trello configuration")] 
    SetConfig(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::SetConfig(config_str) => {
            println!("config_str:{}", config_str);
            let user_id = msg.chat.id.0;
            match parse_trello_config(&config_str, user_id) {
                Ok(config) => {
                    println!("Token: {}", config.token);
                    println!("Key: {}", config.key);
                    println!("Board: {}", config.board);
                    println!("User ID: {}", user_id);
                    let connection_string = env::var("MONGODB_CONNECTION_STRING")
                        .expect("MONGODB_CONNECTION_STRING must be set");
                    let message = match save_config_to_mongodb(&config, &connection_string).await {
                        Ok(_) => {
                            println!("ok");
                            "Trello configuration set successfully!"
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            "Trello configuration set err!"
                        }
                    };

                    bot.send_message(msg.chat.id, message).await?;
                }
                Err(e) => {
                    println!("{}", e);
                    bot.send_message(msg.chat.id, format!("Error setting config: {}", e)).await?;
                }
            }
        }
    }

    Ok(())
}

fn parse_trello_config(config_str: &str, user_id: i64) -> Result<TrelloConfig, String> {
    let parts: Vec<&str> = config_str.split('-').collect();

    if parts.len() != 3 {
        return Err("Invalid format. Expected 'board-key-token'".to_string());
    }

    let (board, key, token) = (parts[0], parts[1], parts[2]);

    if board.is_empty() || key.is_empty() || token.is_empty() {
        return Err("Board, key, and token must not be empty".to_string());
    }

    Ok(TrelloConfig {
        board: board.to_string(),
        key: key.to_string(),
        token: token.to_string(),
        user_id,
    })
}