use crate::config::get_config;
use crate::models::OrderbookDocument;
use mongodb::{
    bson::{doc, Document},
    sync::{Client, Collection},
};

struct Collections {
    orderbook: &'static str,
    money_market_events: &'static str,
    money_market_metadata: &'static str,
    user_positions: &'static str,
    reserve_tokens: &'static str,
    orderbook_metadata: &'static str,
    wallet_factory_events: &'static str,
    intent_events: &'static str,
    eventlog_progress_metadata: &'static str,
}

impl Collections {
    fn new() -> Self {
        Collections {
            orderbook: "orderbook",
            money_market_events: "moneyMarketEvents",
            money_market_metadata: "money_market_metadata",
            user_positions: "user_positions",
            reserve_tokens: "reserve_tokens",
            orderbook_metadata: "orderbookMetadata",
            wallet_factory_events: "walletFactoryEvents",
            intent_events: "intentEvents",
            eventlog_progress_metadata: "eventLogProgressMetadata",
        }
    }
}

fn get_collections_config() -> Collections {
    Collections::new()
}

struct Database {
    client: Client,
}

impl Database {
    fn new() -> Self {
        let uri = get_config().connection_string();
        let client = match Client::with_uri_str(uri) {
            Ok(c) => c,
            Err(e) => panic!("Failed to connect to MongoDB: {}", e),
        };

        Database { client }
    }

    fn database(&self) -> mongodb::sync::Database {
        let db_name = get_config().database_name();
        self.client.database(&db_name)
    }
}

// Simple function-level initialization - each function creates its own connection
// This is actually fine for most use cases since MongoDB Client is designed to be efficient
fn get_db() -> Database {
    Database::new()
}

pub fn get_collections() -> Vec<String> {
    let mut collection_names: Vec<String> = vec![];
    let db = get_db();
    let database: mongodb::sync::Database = db.database();

    match database.list_collections().run() {
        Ok(cursor) => {
            for doc_result in cursor {
                match doc_result {
                    Ok(doc) => {
                        let name: String = doc.name;
                        collection_names.push(name);
                    }
                    Err(e) => panic!("Failed to get collection name: {}", e),
                }
            }
        }
        Err(e) => panic!("Failed to list collections: {}", e),
    };

    // dbg!(&collection_names);
    collection_names
}

pub fn get_orderbook() -> Result<Vec<OrderbookDocument>, mongodb::error::Error> {
    let collection = get_db()
        .database()
        .collection(get_collections_config().orderbook);
    let mut docs: Vec<OrderbookDocument> = vec![];
    match collection.find(doc! {}).run() {
        Ok(cursor) => {
            for doc_result in cursor {
                match doc_result {
                    Ok(doc) => docs.push(doc),
                    Err(e) => {
                        eprintln!("Error getting OrderbookDocument. {}", e);
                        return Err(e);
                    }
                };
            }
        }
        Err(e) => {
            eprintln!("Error finding OrderbookDocument. {}", e);
            return Err(e);
        }
    };
    Ok(docs)
}
