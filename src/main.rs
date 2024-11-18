mod config;
mod error;
mod models;
mod services;
mod handlers;
mod utils;

use crate::config::Config;
use crate::utils::scheduler::run_scheduler;
use dotenv::dotenv;
use teloxide::prelude::*;
use crate::services::database;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = Arc::new(Config::new().expect("Failed to load configuration"));

    let mongodb_client = database::connect_to_mongodb(&config.mongodb_uri).await?;
    let bot = Bot::new(&config.telegram_bot_token);

    let handler = Update::filter_message().branch(
        dptree
            ::entry()
            .filter_command::<handlers::command::Command>()
            .endpoint(handlers::command::answer)
    );

    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![mongodb_client])
        .enable_ctrlc_handler()
        .build();

    let dispatcher_future = dispatcher.dispatch();
    let scheduler_future = run_scheduler(config.clone());

    tokio::join!(dispatcher_future, scheduler_future);

    Ok(())
}
