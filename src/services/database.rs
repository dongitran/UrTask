use crate::error::AppError;
use crate::models::trello::TrelloConfig;
use mongodb::{ Client, options::ClientOptions, Collection };
use mongodb::bson::{ doc, DateTime, Document };
use futures_util::stream::TryStreamExt;
use chrono::{
    NaiveDate,
    NaiveTime,
    DateTime as ChronoDateTime,
    Local,
    TimeZone,
    Utc,
    NaiveDateTime,
    Duration,
    FixedOffset,
};
use serde::{ Serialize, Deserialize };
use std::ops::Sub;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigLog {
    pub user_id: i64,
    pub board: String,
    pub key: String,
    pub token: String,
    pub timestamp: ChronoDateTime<Utc>,
}

pub async fn connect_to_mongodb(uri: &str) -> Result<Client, AppError> {
    let client_options = ClientOptions::parse(uri).await?;
    let client = Client::with_options(client_options)?;
    Ok(client)
}

pub async fn get_trello_configs(client: &Client) -> Result<Vec<TrelloConfig>, AppError> {
    let database = client.database("urtask");
    let collection: Collection<TrelloConfig> = database.collection("trello_configs");
    let mut cursor = collection.find(None, None).await?;

    let mut configs = Vec::new();
    while let Some(config) = cursor.try_next().await? {
        configs.push(config);
    }

    Ok(configs)
}

pub async fn get_trello_config(
    client: &Client,
    user_id: i64
) -> Result<Option<TrelloConfig>, AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<TrelloConfig>("trello_configs");

    let filter = doc! { "userId": user_id };
    let result = collection.find_one(filter, None).await?;

    Ok(result)
}

pub async fn save_trello_config(client: &Client, config: &TrelloConfig) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<TrelloConfig>("trello_configs");

    let filter = doc! { "userId": config.user_id };
    let update = doc! { "$set": bson::to_document(config)? };
    let options = mongodb::options::UpdateOptions::builder().upsert(true).build();

    collection.update_one(filter, update, options).await?;

    Ok(())
}

pub async fn save_reported_task(client: &Client, task_id: &str) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection: Collection<Document> = database.collection("reported_tasks");

    let doc = doc! {
        "task_id": task_id,
        "reported_at": DateTime::now(),
    };

    collection.insert_one(doc, None).await?;
    Ok(())
}

pub async fn is_task_reported(client: &Client, task_id: &str) -> Result<bool, AppError> {
    let database = client.database("urtask");
    let collection: Collection<Document> = database.collection("reported_tasks");

    let filter = doc! { "task_id": task_id };
    let result = collection.find_one(filter, None).await?;

    Ok(result.is_some())
}

pub async fn log_error(client: &Client, error_message: &str) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection: Collection<Document> = database.collection("error_logs");

    let doc = doc! {
        "message": error_message,
        "timestamp": DateTime::now(),
    };

    collection.insert_one(doc, None).await?;
    Ok(())
}

pub async fn log_config_change(client: &Client, log_entry: &ConfigLog) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<ConfigLog>("config_logs");

    collection.insert_one(log_entry, None).await?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInteractionLog {
    pub user_id: i64,
    pub user_name: String,
    pub command: String,
    pub timestamp: ChronoDateTime<Utc>,
}

pub async fn log_user_interaction(
    client: &Client,
    log: UserInteractionLog
) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<UserInteractionLog>("user_interaction_logs");

    collection.insert_one(log, None).await?;
    Ok(())
}

pub async fn has_reported_today(client: &Client, user_id: i64) -> Result<bool, AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<Document>("manual_reports");

    let now = Utc::now();
    let today_start = now
        .with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap())
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let utc_start = Utc.from_local_datetime(&today_start).unwrap().sub(Duration::hours(7));

    let filter =
        doc! {
        "user_id": user_id,
        "reported_at": {
            "$gte": DateTime::from_chrono(utc_start),
            "$lt": DateTime::from_chrono(utc_start + Duration::hours(24))
        }
    };

    let result = collection.find_one(filter, None).await?;
    Ok(result.is_some())
}

pub async fn log_manual_report(client: &Client, user_id: i64) -> Result<(), AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<Document>("manual_reports");

    let doc = doc! {
        "user_id": user_id,
        "reported_at": DateTime::now()
    };

    collection.insert_one(doc, None).await?;
    Ok(())
}

pub async fn get_today_reported_tasks(client: &Client) -> Result<Vec<String>, AppError> {
    let database = client.database("urtask");
    let collection = database.collection::<Document>("reported_tasks");

    let now = Utc::now();
    let today_start = now
        .with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap())
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let utc_start = Utc.from_local_datetime(&today_start).unwrap().sub(Duration::hours(7));

    let start_date = mongodb::bson::DateTime::from_chrono(utc_start);

    println!("Start date: {:?}", start_date);

    let filter = doc! {
        "reported_at": {
            "$gte": start_date
        }
    };

    let mut cursor = collection.find(filter, None).await?;
    let mut task_ids = Vec::new();

    while let Some(doc) = cursor.try_next().await? {
        if let Ok(task_id) = doc.get_str("task_id") {
            task_ids.push(task_id.to_string());
        }
    }

    Ok(task_ids)
}
