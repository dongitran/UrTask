use mongodb::{Client as MongoClient, options::ClientOptions};
use mongodb::bson::{doc, Document, DateTime};
use mongodb::options::UpdateOptions;
use chrono::Utc;
use crate::trello::TrelloConfig;
use std::error::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct ConfigLog {
    user_id: i64,
    board: String,
    key: String,
    token: String,
    timestamp: DateTime,
}

pub async fn save_config_to_mongodb(
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
        let update = doc! {
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

    // Log the config change
    log_config_change(config, &client).await?;

    Ok(())
}

async fn log_config_change(config: &TrelloConfig, client: &MongoClient) -> Result<(), Box<dyn Error>> {
    let database = client.database("urtask");
    let log_collection = database.collection::<ConfigLog>("config_logs");

    let log_entry = ConfigLog {
        user_id: config.user_id,
        board: config.board.clone(),
        key: config.key.clone(),
        token: config.token.clone(),
        timestamp: DateTime::now(),
    };

    log_collection.insert_one(log_entry, None).await?;

    Ok(())
}