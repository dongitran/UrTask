use crate::config::Config;
use crate::utils::scheduler::run_scheduler;
use dotenv::dotenv;
use teloxide::prelude::*;
use crate::services::database;
use std::sync::Arc;
use tokio::select;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = Arc::new(Config::new().expect("Failed to load configuration"));

    let mongodb_client = database::connect_to_mongodb(&config.mongodb_uri).await?;
    let bot = Bot::new(&config.telegram_bot_token);

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<handlers::command::Command>()
                .endpoint(handlers::command::answer),
        );

    let dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![mongodb_client])
        .enable_ctrlc_handler()
        .build();

    let scheduler_handle = tokio::spawn(run_scheduler(config.clone()));
    
    select! {
        _ = dispatcher.dispatch() => println!("Bot stopped"),
        _ = scheduler_handle => println!("Scheduler stopped"),
    }

    Ok(())
}