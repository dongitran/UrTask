use mongodb::{Client as MongoClient, options::ClientOptions};
use mongodb::bson::{doc, Document};
use mongodb::options::UpdateOptions;
use chrono::Utc;
use crate::trello::TrelloConfig;
use std::error::Error;

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

    Ok(())
}