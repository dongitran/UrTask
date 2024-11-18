use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use crate::services::{ database, trello };
use crate::models::trello::TrelloConfig;
use crate::models::trello::TrelloCard;
use crate::error::AppError;
use chrono::Utc;
use std::fmt;
use crate::services::telegram;
use crate::utils::scheduler;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "UrTask Bot supports the following commands:")]
pub enum Command {
    #[command(description = "Start the bot and get a welcome message")]
    Start,
    #[command(description = "Show the list of available commands")]
    Help,
    #[command(
        description = "Set up your Trello configuration. Usage: /setconfig board_id-api_key-api_token"
    )] SetConfig(String),
    #[command(description = "Test your current Trello configuration")]
    TestConfig,
    #[command(description = "Get an immediate report of your tasks")]
    ReportNow,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Start => write!(f, "start"),
            Command::Help => write!(f, "help"),
            Command::SetConfig(_) => write!(f, "setconfig"),
            Command::TestConfig => write!(f, "testconfig"),
            Command::ReportNow => write!(f, "reportnow"),
        }
    }
}

pub async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    mongodb_client: mongodb::Client
) -> ResponseResult<()> {
    // Log user interaction
    let user_id = msg
        .from()
        .map(|user| user.id.0 as i64)
        .unwrap_or(0);
    let user_name = msg
        .from()
        .map(|user| user.full_name())
        .unwrap_or_else(|| "Unknown User".to_string());
    let command_name = cmd.to_string();

    let log = database::UserInteractionLog {
        user_id,
        user_name,
        command: command_name,
        timestamp: Utc::now(),
    };

    if let Err(e) = database::log_user_interaction(&mongodb_client, log).await {
        eprintln!("Failed to log user interaction: {:?}", e);
    }

    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "👋 Welcome to UrTask Bot!\n\n\
            🚀 UrTask is an automated tool that bridges Trello and Telegram, providing daily summaries of your Trello activities.\n\n\
            🔧 To get started, you need to set up your Trello configuration using the /setconfig command.\n\n\
            📌 Use the following format:\n\
            /setconfig your_board_id-your_api_key-your_api_token\n\n\
            🔑 You can find your Trello API key and token at: https://trello.com/app-key\n\n\
            📋 Important: Your Trello board must have three lists named exactly:\n\
            • ToDo\n\
            • Doing\n\
            • Done\n\
            UrTask will use these lists to generate your daily reports.\n\n\
            ❓ Use /help to see all available commands."
            ).await?;
        }
        Command::Help => {
            bot.send_message(
                msg.chat.id,
                "🤖 UrTask Bot helps you manage your Trello tasks and send daily reports. Available commands:\n\n\
              🚀 /start - Get started with UrTask Bot\n\n\
              ❓ /help - Show this help message\n\n\
              ⚙️ /setconfig - Set up your Trello configuration.\n Usage: /setconfig board_id-api_key-api_token\n\n\
              🧪 /testconfig - Test your current Trello configuration\n\n\
              📊 /reportnow - Get an immediate report of your tasks\n\n\n\
              📅 After setting up, you'll receive:\n\
              • Daily reports at 9:35 AM (GMT+7)\n\
              • Morning reminders at 9:15 AM (GMT+7)\n\
              • Evening reminders at 5:00 PM (GMT+7)\n\n\
              All scheduled messages are sent on weekdays (Monday to Friday) only."
            ).await?;
        }
        Command::SetConfig(config_str) => {
            let user_id = msg.chat.id.0;
            match parse_trello_config(&config_str, user_id) {
                Ok(config) => {
                    match database::save_trello_config(&mongodb_client, &config).await {
                        Ok(_) => {
                            bot.send_message(
                                msg.chat.id,
                                "✅ Trello configuration saved. Testing configuration..."
                            ).await?;
                            test_trello_config(&bot, msg.chat.id, &config).await?;
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            bot.send_message(
                                msg.chat.id,
                                "❌ Error setting Trello configuration. Please try again later."
                            ).await?;
                        }
                    }
                }
                Err(e) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("❌ Error setting config: {}. Please use the format: /setconfig board_id-api_key-api_token", e)
                    ).await?;
                }
            }
        }
        Command::TestConfig => {
            let user_id = msg.chat.id.0;
            match database::get_trello_config(&mongodb_client, user_id).await {
                Ok(Some(config)) => {
                    test_trello_config(&bot, msg.chat.id, &config).await?;
                }
                Ok(None) => {
                    bot.send_message(
                        msg.chat.id,
                        "❌ No Trello configuration found. Please set up your configuration using /setconfig"
                    ).await?;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "❌ Error retrieving Trello configuration. Please try again later."
                    ).await?;
                }
            }
        }
        Command::ReportNow => {
            let user_id = msg.chat.id.0;
            match database::get_trello_config(&mongodb_client, user_id).await {
                Ok(Some(config)) => {
                    let now = chrono::Local::now();
                    let cutoff_time = now.date_naive().and_hms_opt(9, 35, 0).unwrap();

                    if now.naive_local() < cutoff_time {
                        if let Err(e) = database::log_manual_report(&mongodb_client, user_id).await {
                            eprintln!("Failed to log manual report: {}", e);
                        }
                    }

                    if
                        let Err(e) = scheduler::process_user_config(
                            &config,
                            &bot.token(),
                            &mongodb_client,
                            true
                        ).await
                    {
                        bot.send_message(
                            msg.chat.id,
                            format!("Error generating report: {}", e)
                        ).await?;
                    }
                }
                Ok(None) => {
                    bot.send_message(
                        msg.chat.id,
                        "No Trello configuration found. Please set up using /setconfig"
                    ).await?;
                }
                Err(e) => {
                    bot.send_message(
                        msg.chat.id,
                        format!("Error retrieving configuration: {}", e)
                    ).await?;
                }
            }
        }
    }
    Ok(())
}

async fn save_config(client: &mongodb::Client, config: &TrelloConfig) -> Result<(), AppError> {
    // Save the config
    database::save_trello_config(client, config).await?;

    // Log the config change
    let log_entry = database::ConfigLog {
        user_id: config.user_id,
        board: config.board.clone(),
        key: config.key.clone(),
        token: config.token.clone(),
        timestamp: chrono::Utc::now(),
    };

    database::log_config_change(client, &log_entry).await?;

    Ok(())
}

fn parse_trello_config(config_str: &str, user_id: i64) -> Result<TrelloConfig, String> {
    let parts: Vec<&str> = config_str
        .split('-')
        .collect::<Vec<&str>>()
        .iter()
        .map(|&s| s.trim())
        .filter(|&s| !s.is_empty())
        .collect();

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

async fn test_trello_config(
    bot: &Bot,
    chat_id: ChatId,
    config: &TrelloConfig
) -> ResponseResult<()> {
    match trello::check_trello_lists(config).await {
        Ok((todo_id, doing_id, done_id)) => {
            let message = format!(
                "✅ Trello configuration is valid!\n\nBoard ID: {}\nToDo List ID: {}\nDoing List ID: {}\nDone List ID: {}",
                config.board,
                todo_id,
                doing_id,
                done_id
            );
            bot.send_message(chat_id, message).await?;
        }
        Err(e) => {
            let error_message = format!("❌ Error testing Trello configuration: {}", e);
            bot.send_message(chat_id, error_message).await?;
        }
    }
    Ok(())
}
