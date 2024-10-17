use crate::config::Config;
use crate::services::{ database, trello, telegram };
use crate::error::AppError;
use crate::models::trello::{ TrelloConfig, TrelloCard };
use std::sync::Arc;
use chrono::{Utc, FixedOffset, TimeZone, Local, Datelike};
use tokio_cron_scheduler::{ JobScheduler, Job };
use rand::seq::SliceRandom;
use chrono::Weekday;

pub async fn run_scheduler(config: Arc<Config>) -> Result<(), AppError> {
    let scheduler = JobScheduler::new().await?;
    let gmt7 = FixedOffset::east_opt(7 * 3600).expect("Invalid timezone");

    scheduler.add(
        Job::new_async("0 0 3 * * 1-5", {
            let config = config.clone();
            move |_, _| {
                let config = config.clone();
                Box::pin(async move {
                    let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                    println!("Running cron job at {:?}", now);
                    if let Err(e) = run_cron_job(&config).await {
                        eprintln!("Error in cron job: {:?}", e);
                    }
                })
            }
        })?
    ).await?;

    scheduler.add(
        Job::new_async("0 15 2 * * 1-5", {
            let config = config.clone();
            move |_, _| {
                let config = config.clone();
                Box::pin(async move {
                    let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                    println!("Running morning reminder job at {:?}", now);
                    if let Err(e) = run_reminder_job(&config, "morning").await {
                        eprintln!("Error in morning reminder job: {:?}", e);
                    }
                })
            }
        })?
    ).await?;

    scheduler.add(
        Job::new_async("0 0 10 * * 1-5", {
            let config = config.clone();
            move |_, _| {
                let config = config.clone();
                Box::pin(async move {
                    let now = gmt7.from_utc_datetime(&Utc::now().naive_utc());
                    println!("Running evening reminder job at {:?}", now);
                    if let Err(e) = run_reminder_job(&config, "evening").await {
                        eprintln!("Error in evening reminder job: {:?}", e);
                    }
                })
            }
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
        let message = generate_reminder_message(time_of_day);

        if
            let Err(e) = telegram::send_message(
                &config.telegram_bot_token,
                trello_config.user_id,
                &message
            ).await
        {
            eprintln!("Error sending reminder to user {}: {:?}", trello_config.user_id, e);
        }
    }

    Ok(())
}

fn generate_reminder_message(time_of_day: &str) -> String {
    let now = chrono::Local::now();
    let weekday = now.weekday();

    let (greeting, action) = match time_of_day {
        "morning" => (get_morning_greeting(&weekday), get_morning_action()),
        "evening" => (get_evening_greeting(&weekday), get_evening_action()),
        _ => ("Hello".to_string(), "It's time to update your Trello board!".to_string()),
    };

    format!("{} {}\n{}", greeting, get_emoji(), action)
}

fn get_morning_greeting(weekday: &Weekday) -> String {
    let greetings = match weekday {
        Weekday::Mon =>
            vec![
                "Monday motivation!",
                "New week, new goals!",
                "Monday blues? Not for you!",
                "Ready to conquer this week?",
                "Monday: A fresh start!",
                "This week's going to be amazing!",
                "Monday: Let's make it count!",
                "Embrace the Monday magic!",
                "Week mode: Activated!",
                "Monday's here! Let's rock!",
                "New week, endless possibilities!",
                "Monday: Your weekly reboot!",
                "Monday hustle begins!",
                "This week's your canvas, paint it well!",
                "Monday: Turning coffee into productivity!"
            ],
        Weekday::Fri =>
            vec![
                "TGIF!",
                "Friday vibes!",
                "Last workday of the week!",
                "Friday: The golden hour of the week!",
                "Weekend is on the horizon!",
                "Friday: Make it awesome!",
                "Almost there! Finish strong!",
                "Friday: The day before Saturday!",
                "Fri-nally!",
                "Friday: Let's wrap this week up nicely!",
                "Friday mood: Activated!",
                "One last push before the weekend!",
                "Friday: The week's victory lap!",
                "Countdown to weekend: Activated!",
                "Friday: Let's end this week with a bang!"
            ],
        _ =>
            vec![
                "Rise and shine!",
                "Good morning!",
                "Hello, sunshine!",
                "Top of the morning to you!",
                "Wakey wakey!",
                "A new day has dawned!",
                "Morning glory!",
                "Hello, new day!",
                "Time to rise and conquer!",
                "Good day to you!",
                "Morning has broken!",
                "Up and at 'em!",
                "Greetings, early bird!",
                "A fresh start awaits!",
                "Welcome to a brand new day!"
            ],
    };
    greetings.choose(&mut rand::thread_rng()).unwrap().to_string()
}

fn get_evening_greeting(weekday: &Weekday) -> String {
    let greetings = match weekday {
        Weekday::Fri =>
            vec![
                "Happy Friday evening!",
                "Weekend is knocking!",
                "Friday night lights!",
                "TGIF evening edition!",
                "Friday night, best night!",
                "Weekend countdown: Hours!",
                "Friday PM: The best time of the week!",
                "It's Fri-yay night!",
                "Weekend mode: Activating...",
                "Friday evening: The golden hours!",
                "Welcome to the weekend's eve!",
                "Friday PM: Time to unwind!",
                "The week's grand finale is here!",
                "Friday night: Let the good times roll!",
                "Congrats! You've made it to Friday evening!"
            ],
        _ =>
            vec![
                "Good evening!",
                "Wrapping up the day?",
                "Hello again!",
                "Evening greetings!",
                "How was your day?",
                "Evening has arrived!",
                "Winding down time!",
                "Evening check-in!",
                "Day's end is near!",
                "Evening vibes!",
                "Twilight greetings!",
                "As the day concludes...",
                "Evening reflection time!",
                "Day's wrap-up!",
                "Evening has fallen!"
            ],
    };
    greetings.choose(&mut rand::thread_rng()).unwrap().to_string()
}

fn get_morning_action() -> String {
    let actions = vec![
        "Time to plan your day in Trello!",
        "Let's crush some tasks today!",
        "Your Trello board is waiting for updates!",
        "Start your day right - update your Trello!",
        "New day, new tasks. Update that Trello board!",
        "Kick off your day with a Trello check-in!",
        "Set your intentions for the day on Trello!",
        "Map out your day for success on Trello!",
        "Begin with the end in mind - plan on Trello!",
        "Your day's blueprint awaits on Trello!",
        "Chart your course for the day on Trello!",
        "Trello's ready for your day's game plan!",
        "Align your day with your goals on Trello!",
        "Start strong - organize your day on Trello!",
        "Your Trello board is your day's command center!",
        "Plan, prioritize, conquer - start with Trello!",
        "Morning Trello update: Your recipe for a productive day!",
        "Fuel your day with a well-organized Trello board!",
        "Your day's potential is waiting on Trello!",
        "Seize the day - starting with your Trello board!",
        "Morning Trello ritual: Setting you up for success!",
        "Kickstart your productivity on Trello!",
        "Your Trello board: The launchpad for a great day!",
        "Design your perfect day on Trello!",
        "Trello morning update: Your daily success habit!"
    ];
    actions.choose(&mut rand::thread_rng()).unwrap().to_string()
}

fn get_evening_action() -> String {
    let actions = vec![
        "Don't forget to update your Trello before signing off!",
        "Reflect on your day's achievements in Trello.",
        "Wrap up your day with a Trello update!",
        "Tomorrow you'll thank yourself for updating Trello now.",
        "One last thing before you go - Trello update time!",
        "Cap off your day with a Trello check-in!",
        "Review and reset your Trello board for tomorrow.",
        "Close your day strong with a Trello update!",
        "Set yourself up for success tomorrow - update Trello now!",
        "Your future self will thank you for updating Trello!",
        "End-of-day Trello ritual: Your key to continuous progress!",
        "Before you clock out, clock in with your Trello board!",
        "Cement today's progress with a Trello update!",
        "Your day's not complete without a final Trello check!",
        "Prepare for tomorrow's success with a Trello evening update!",
        "Round off your day with a Trello review!",
        "A quick Trello update now saves time tomorrow!",
        "Evening Trello sync: Aligning today with tomorrow!",
        "Capture today's achievements and tomorrow's goals on Trello!",
        "Your evening Trello update: The bridge to tomorrow's success!",
        "Close the loop on today's tasks with a Trello update!",
        "Evening Trello ritual: Your productivity nightcap!",
        "Lay the groundwork for tomorrow on Trello tonight!",
        "Trello evening update: Your daily victory lap!",
        "Bookend your day with a final Trello check-in!"
    ];
    actions.choose(&mut rand::thread_rng()).unwrap().to_string()
}

fn get_emoji() -> &'static str {
    let emojis = vec![
        "🚀",
        "💪",
        "✨",
        "🌟",
        "🔥",
        "📊",
        "🎯",
        "🏆",
        "💡",
        "⚡",
        "🌈",
        "🌻",
        "🌞",
        "🌠",
        "🏅",
        "🎉",
        "🎊",
        "🥇",
        "🏋️",
        "🧠",
        "🌺",
        "🍀",
        "🌸",
        "🌼",
        "🌿"
    ];
    emojis.choose(&mut rand::thread_rng()).unwrap()
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
