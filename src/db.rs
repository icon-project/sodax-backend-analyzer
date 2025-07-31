use crate::config::get_config;
use crate::models::{OrderbookDocument, ReserveTokenDocument};
// For async iteration over cursor
use futures::stream::StreamExt;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};

struct Collections {
    orderbook: &'static str,
    #[allow(dead_code)]
    money_market_events: &'static str,
    #[allow(dead_code)]
    money_market_metadata: &'static str,
    #[allow(dead_code)]
    user_positions: &'static str,
    reserve_tokens: &'static str,
    #[allow(dead_code)]
    orderbook_metadata: &'static str,
    #[allow(dead_code)]
    wallet_factory_events: &'static str,
    #[allow(dead_code)]
    intent_events: &'static str,
    #[allow(dead_code)]
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
    async fn new() -> Self {
        let uri = get_config().connection_string();
        let options = ClientOptions::parse(uri).await.unwrap();
        let client = match Client::with_options(options) {
            Ok(c) => c,
            Err(e) => panic!("Failed to connect to MongoDB: {}", e),
        };

        Database { client }
    }

    fn database(&self) -> mongodb::Database {
        let db_name = get_config().database_name();
        self.client.database(&db_name)
    }
}

// Simple function-level initialization - each function creates its own connection
// This is actually fine for most use cases since MongoDB Client is designed to be efficient
async fn get_db() -> Database {
    Database::new().await
}

pub async fn get_collections() -> Vec<String> {
    let mut collection_names: Vec<String> = vec![];
    let db = get_db().await;
    let database: mongodb::Database = db.database();

    match database.list_collections().await {
        Ok(mut cursor) => {
            while let Some(doc_result) = cursor.next().await {
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

pub async fn get_orderbook() -> Result<Vec<OrderbookDocument>, mongodb::error::Error> {
    let collection: Collection<OrderbookDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().orderbook);
    let mut docs: Vec<OrderbookDocument> = vec![];
    match collection.find(doc! {}).await {
        Ok(mut cursor) => {
            while let Some(doc_result) = cursor.next().await {
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

#[derive(Debug)]
enum ReserveTokenField {
    Reserve,
    AToken,
    VariableDebtToken,
}

async fn find_reserve_for_token(
    token: &str,
    token_type: ReserveTokenField,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    let collection: Collection<ReserveTokenDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().reserve_tokens);

    let filter = match token_type {
        ReserveTokenField::Reserve => doc! { "reserveAddress": token },
        ReserveTokenField::AToken => doc! { "aTokenAddress": token },
        ReserveTokenField::VariableDebtToken => doc! { "variableDebtTokenAddress": token },
    };

    match collection.find_one(filter).await {
        Ok(doc) => Ok(doc),
        Err(e) => {
            eprintln!("Error finding reserve for token {}: {}", token, e);
            Err(e)
        }
    }
}

pub async fn get_reserve_data_for_reserve_token(
    address: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(address, ReserveTokenField::Reserve).await
}

pub async fn get_reserve_data_for_a_token(
    address: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(address, ReserveTokenField::AToken).await
}

pub async fn get_reserve_data_for_variable_debt_token(
    address: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(address, ReserveTokenField::VariableDebtToken).await
}

pub async fn find_all_reserves() -> Result<Vec<ReserveTokenDocument>, mongodb::error::Error> {
    let collection: Collection<ReserveTokenDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().reserve_tokens);
    let mut reserves: Vec<ReserveTokenDocument> = vec![];
    let mut cursor = collection.find(doc! {}).await?;

    while let Some(doc_result) = cursor.next().await {
        match doc_result {
            Ok(doc) => reserves.push(doc),
            Err(e) => {
                eprintln!("Error getting ReserveTokenDocument. {}", e);
                return Err(e);
            }
        };
    }
    Ok(reserves)
}

pub async fn find_all_reserve_addresses() -> Vec<String> {
    let reserves = find_all_reserves().await.unwrap_or_else(|e| {
        eprintln!("Failed to find all reserves: {}", e);
        vec![]
    });

    let reserve_addresses: Vec<String> =
        reserves.iter().map(|r| r.reserveAddress.clone()).collect();

    // dbg!(&reserve_addresses);
    reserve_addresses
}

pub async fn find_all_a_token_addresses() -> Vec<String> {
    let reserves = find_all_reserves().await.unwrap_or_else(|e| {
        eprintln!("Failed to find all reserves: {}", e);
        vec![]
    });

    let a_token_addresses: Vec<String> = reserves.iter().map(|r| r.aTokenAddress.clone()).collect();

    // dbg!(&a_token_addresses);
    a_token_addresses
}

pub async fn find_all_variable_debt_token_addresses() -> Vec<String> {
    let reserves = find_all_reserves().await.unwrap_or_else(|e| {
        eprintln!("Failed to find all reserves: {}", e);
        vec![]
    });

    let variable_debt_token_addresses: Vec<String> = reserves
        .iter()
        .map(|r| r.variableDebtTokenAddress.clone())
        .collect();

    // dbg!(&variable_debt_token_addresses);
    variable_debt_token_addresses
}
