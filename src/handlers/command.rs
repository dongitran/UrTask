use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use crate::services::database;
use crate::models::trello::TrelloConfig;
use crate::error::AppError;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "set Trello configuration")] 
    SetConfig(String),
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command, mongodb_client: mongodb::Client) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::SetConfig(config_str) => {
            let user_id = msg.chat.id.0;
            match parse_trello_config(&config_str, user_id) {
                Ok(config) => {
                    let message = match save_config(&mongodb_client, &config).await {
                        Ok(_) => "Trello configuration set successfully!",
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            "Error setting Trello configuration. Please try again later."
                        }
                    };
                    bot.send_message(msg.chat.id, message).await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Error setting config: {}. Please use the format 'board-key-token'.", e)).await?;
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
    let parts: Vec<&str> = config_str.split('-').collect::<Vec<&str>>()
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