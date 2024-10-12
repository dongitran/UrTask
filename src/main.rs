use reqwest::Client;
use mongodb::{ Client as MongoClient, options::ClientOptions };
use futures_util::stream::TryStreamExt;
use std::error::Error;
use serde_json::json;
use clokwerk::{ Scheduler, TimeUnits };
use chrono::{ NaiveTime, FixedOffset, DateTime, Utc, Datelike };
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use teloxide::{ prelude::*, utils::command::BotCommands };
use serde::{ Serialize, Deserialize };
use bson;
use mongodb::options::UpdateOptions;
use mongodb::bson::{ doc, Document };
use std::env;
use dotenv::dotenv;

#[derive(Deserialize, Debug)]
struct TrelloList {
    id: String,
    name: String,
    closed: bool,
    idBoard: String,
    pos: f64,
}

#[derive(Deserialize, Debug)]
struct TrelloCard {
    id: String,
    name: String,
    idList: String,
}

#[derive(Debug)]
struct TrelloCheckResult {
    status: bool,
    todo_id: Option<String>,
    doing_id: Option<String>,
    done_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TrelloConfig {
    board: String,
    key: String,
    token: String,
    #[serde(rename = "userId")]
    user_id: i64,
}

async fn send_message(
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

async fn check_trello_lists(
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
                "ToDo" => {
                    result.todo_id = Some(list.id.clone());
                }
                "Doing" => {
                    result.doing_id = Some(list.id.clone());
                }
                "Done" => {
                    result.done_id = Some(list.id.clone());
                }
                _ => {}
            }
        }

        if result.todo_id.is_some() && result.doing_id.is_some() && result.done_id.is_some() {
            result.status = true;
            return Ok(result);
        } else {
            return Err("Danh sách KHÔNG có đủ 3 tên 'ToDo', 'Doing', và 'Done'.".to_string());
        }
    } else {
        return Err(format!("Lỗi: {}", response.status()));
    }
}

async fn get_trello_cards(key: &str, token: &str, board: &str) -> Result<Vec<TrelloCard>, String> {
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

fn map_cards_to_lists(
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

async fn cron_job() -> Result<(), Box<dyn std::error::Error>> {
    let connection_string = env
        ::var("MONGODB_CONNECTION_STRING")
        .expect("MONGODB_CONNECTION_STRING must be set");
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");

    let client_options = ClientOptions::parse(&connection_string).await?;
    let client = MongoClient::with_options(client_options)?;
    let database = client.database("urtask");
    let collection = database.collection::<TrelloConfig>("trello_configs");

    let mut cursor = collection.find(None, None).await?;

    while let Some(config) = cursor.try_next().await? {
        match check_trello_lists(&config.key, &config.token, &config.board).await {
            Ok(result) => {
                let cards = get_trello_cards(
                    &config.key,
                    &config.token,
                    &config.board
                ).await.unwrap();
                let (todo_cards, doing_cards, done_cards) = map_cards_to_lists(result, cards);

                println!("User ID: {}", config.user_id);
                println!("ToDo Cards: {:?}", todo_cards);
                println!("Doing Cards: {:?}", doing_cards);
                println!("Done Cards: {:?}", done_cards);

                let today = Utc::now().naive_utc().date();
                let yesterday = today.pred_opt().expect("Failed to get previous day");

                let mut message = format!("Hôm qua ({}):\n", yesterday);
                for card in done_cards {
                    message.push_str(&format!("- {}\n", card.name));
                }

                message.push_str("Hôm nay:\n");
                for card in doing_cards {
                    message.push_str(&format!("- {}\n", card.name));
                }
                for card in todo_cards {
                    message.push_str(&format!("- {}\n", card.name));
                }

                send_message(&bot_token, config.user_id, &message).await?;
            }
            Err(err) => {
                println!("Error for user {}: {}", config.user_id, err);
                let error_message =
                    "Lỗi: Không thể tìm thấy đủ 3 danh sách 'ToDo', 'Doing' và 'Done' trong bảng Trello của bạn. Vui lòng thiết lập các danh sách này để bot hoạt động chính xác.";
                send_message(&bot_token, config.user_id, error_message).await?;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let scheduler = Arc::new(
        Mutex::new(Scheduler::with_tz(FixedOffset::east_opt(7 * 3600).expect("Invalid offset")))
    );

    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let bot = Bot::new(&bot_token);

    let bot_handle = tokio::spawn(Command::repl(bot, answer));

    {
        let scheduler_clone = Arc::clone(&scheduler);
        tokio::spawn(async move {
            let mut scheduler = scheduler_clone.lock().await;

            scheduler.every((10).seconds()).run(move || {
                tokio::spawn(async move {
                    if let Err(e) = cron_job().await {
                        eprintln!("Error running cron job: {:?}", e);
                    }
                });
            });

            loop {
                scheduler.run_pending();
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
    #[command(description = "set Trello configuration")] SetConfig(String),
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
                    let connection_string = env
                        ::var("MONGODB_CONNECTION_STRING")
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

async fn save_config_to_mongodb(
    config: &TrelloConfig,
    connection_string: &str
) -> Result<(), Box<dyn Error>> {
    let client_options = ClientOptions::parse(connection_string).await?;
    let client = MongoClient::with_options(client_options)?;
    let database = client.database("urtask");
    let collection = database.collection::<Document>("trello_configs");

    let mut doc = bson::to_document(&config)?;

    let now = Utc::now();

    let filter = doc! { "userId": config.user_id };

    let existing_doc = collection.find_one(filter.clone(), None).await?;

    if existing_doc.is_some() {
        let update =
            doc! {
            "$set": {
                "board": &config.board,
                "key": &config.key,
                "token": &config.token,
                "updatedAt": now
            }
        };
        collection.update_one(filter, update, None).await?;
    } else {
        doc.insert("createdAt", bson::DateTime::from(now));
        doc.insert("updatedAt", bson::DateTime::from(now));

        let update = doc! { "$set": doc };
        let options = UpdateOptions::builder().upsert(true).build();
        collection.update_one(filter, update, options).await?;
    }

    Ok(())
}
