use std::env;

pub struct Config {
    pub mongodb_uri: String,
    pub telegram_bot_token: String,
}

impl Config {
    pub fn new() -> Result<Self, env::VarError> {
        Ok(Config {
            mongodb_uri: env::var("MONGODB_CONNECTION_STRING")?,
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")?,
        })
    }
}