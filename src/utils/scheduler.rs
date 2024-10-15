use crate::config::Config;
use crate::services::{ database, trello, telegram };
use crate::error::AppError;
use crate::models::trello::{ TrelloConfig, TrelloCard };
use std::sync::Arc;
use chrono::{ Utc, FixedOffset, TimeZone, Local };
use tokio_cron_scheduler::{ JobScheduler, Job };

pub async fn run_scheduler(config: Arc<Config>) -> Result<(), AppError> {
    let scheduler = JobScheduler::new().await?;
    let gmt7 = FixedOffset::east_opt(7 * 3600).expect("Invalid timezone");

    scheduler.add(
        Job::new_async("0 35 2 * * *", move |_, _| {
            let config = config.clone();
            Box::pin(async move {
                let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                println!("Running cron job at {:?}", now);
                if let Err(e) = run_cron_job(&config).await {
                    eprintln!("Error in cron job: {:?}", e);
                }
            })
        })?
    ).await?;

    scheduler.add(
        Job::new_async("0 15 2 * * *", move |_, _| {
            let config = config.clone();
            Box::pin(async move {
                let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                println!("Running morning reminder job at {:?}", now);
                if let Err(e) = run_reminder_job(&config, "morning").await {
                    eprintln!("Error in morning reminder job: {:?}", e);
                }
            })
        })?
    ).await?;

    scheduler.add(
        Job::new_async("0 0 10 * * *", move |_, _| {
            let config = config.clone();
            Box::pin(async move {
                let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                println!("Running evening reminder job at {:?}", now);
                if let Err(e) = run_reminder_job(&config, "evening").await {
                    eprintln!("Error in evening reminder job: {:?}", e);
                }
            })
        })?
    ).await?;

    scheduler.start().await?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn run_reminder_job(config: &Config, time_of_day: &str) -> Result<(), AppError> {
    let client = database::connect_to_mongodb(&config.mongodb_uri).await?;
    let trello_configs = database::get_trello_configs(&client).await?;

    for trello_config in trello_configs {
        let message = match time_of_day {
            "morning" =>
                "Good morning! Don't forget to update your Trello board with your tasks for today.",
            "evening" =>
                "Good evening! Please make sure your Trello board is up to date before finishing your day.",
            _ => "It's time to update your Trello board!",
        };

        if
            let Err(e) = telegram::send_message(
                &config.telegram_bot_token,
                trello_config.user_id,
                message
            ).await
        {
            eprintln!("Error sending reminder to user {}: {:?}", trello_config.user_id, e);
        }
    }

    Ok(())
}

async fn run_cron_job(config: &Config) -> Result<(), AppError> {
    let client = database::connect_to_mongodb(&config.mongodb_uri).await?;
    let trello_configs = database::get_trello_configs(&client).await?;

    println!("Found {} Trello configurations", trello_configs.len());

    for trello_config in trello_configs {
        println!("Processing config for user {}", trello_config.user_id);
        match process_user_config(&trello_config, &config.telegram_bot_token, &client).await {
            Ok(_) =>
                println!("Processed user config successfully for user {}", trello_config.user_id),
            Err(e) => {
                let error_message = format!(
                    "Error processing user config for user {}: {:?}",
                    trello_config.user_id,
                    e
                );
                println!("{}", error_message);
                database::log_error(&client, &error_message).await?;
            }
        }
    }

    Ok(())
}

async fn process_user_config(
    config: &TrelloConfig,
    bot_token: &str,
    client: &mongodb::Client
) -> Result<(), AppError> {
    let (todo_id, doing_id, done_id) = trello::check_trello_lists(config).await?;
    println!("List IDs - ToDo: {}, Doing: {}, Done: {}", todo_id, doing_id, done_id);

    let cards = trello::get_trello_cards(config).await?;
    let (todo_cards, doing_cards, done_cards) = trello::map_cards_to_lists(
        cards,
        &todo_id,
        &doing_id,
        &done_id
    );

    println!(
        "Found {} todo, {} doing, and {} done cards for user {}",
        todo_cards.len(),
        doing_cards.len(),
        done_cards.len(),
        config.user_id
    );

    let mut new_done_cards = Vec::new();
    for card in &done_cards {
        if !database::is_task_reported(client, &card.id).await? {
            new_done_cards.push(card);
            database::save_reported_task(client, &card.id).await?;
        }
    }

    println!("Found {} new done cards for user {}", new_done_cards.len(), config.user_id);

    if !new_done_cards.is_empty() || !doing_cards.is_empty() || !todo_cards.is_empty() {
        let message = generate_report_message(&todo_cards, &doing_cards, &new_done_cards);
        println!("Sending message to user {}: {}", config.user_id, message);
        telegram::send_message(bot_token, config.user_id, &message).await?;
        println!("Message sent successfully to user {}", config.user_id);
    } else {
        println!("No changes to report for user {}", config.user_id);
    }

    Ok(())
}

fn generate_report_message(
    _todo_cards: &[TrelloCard],
    doing_cards: &[TrelloCard],
    done_cards: &[&TrelloCard]
) -> String {
    let yesterday = Local::now()
        .date_naive()
        .pred_opt()
        .expect("Invalid date")
        .format("%d/%m")
        .to_string();
    let mut message = format!("Hôm trước ({}):\n", yesterday);

    for card in done_cards {
        message.push_str(&format!("* {}\n", card.name));
    }

    message.push_str("\nHôm nay:\n");
    for card in doing_cards {
        message.push_str(&format!("* {}\n", card.name));
    }

    message
}
