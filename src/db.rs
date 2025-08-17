use crate::config::get_config;
use std::str::FromStr;
use crate::models::{
    OrderbookDocument,
    ReserveTokenDocument,
    UserPositionDocument,
    SolverVolumeDocument,
    SolverVolumeTimestampAndBlock,
    MoneyMarketEventDocument,
    // IntentEventDocument
};
// For async iteration over cursor
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection,
    options::FindOptions,
};
use crate::structs::{Collections, ReserveTokenField};

struct Database {
    client: Client,
}
use alloy::primitives::Address;

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

fn get_collections_config() -> Collections {
    Collections::new()
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
    let docs: Vec<OrderbookDocument> = collect_all(collection).await?;
    Ok(docs)
}

pub async fn get_solver_volume() -> Result<Vec<SolverVolumeDocument>, mongodb::error::Error> {
    let collection: Collection<SolverVolumeDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().solver_volume);
    let docs: Vec<SolverVolumeDocument> = collect_all(collection).await?;
    Ok(docs)
}

pub async fn find_docs_with_non_null_timestamp()
-> Result<Vec<SolverVolumeDocument>, mongodb::error::Error> {
    let collection: Collection<SolverVolumeDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().solver_volume);

    let filter = doc! { "timestamp": { "$exists": true } };
    let docs: Vec<SolverVolumeDocument> = collect_all_with_filter(collection, filter).await?;
    Ok(docs)
}

pub async fn find_timestamp_and_block_from_solver_volume()
-> Result<Vec<SolverVolumeTimestampAndBlock>, mongodb::error::Error> {
    let collection: Collection<SolverVolumeTimestampAndBlock> = get_db()
        .await
        .database()
        .collection(get_collections_config().solver_volume);

    let filter = doc! { "timestamp": { "$exists": true } };
    let projection = doc! {"_id":1, "timestamp": 1, "blockNumber": 1 };
    let docs: Vec<SolverVolumeTimestampAndBlock> =
        collect_helper(collection, filter, Some(projection)).await?;
    Ok(docs)
}

pub async fn find_all_reserves() -> Result<Vec<ReserveTokenDocument>, mongodb::error::Error> {
    let collection: Collection<ReserveTokenDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().reserve_tokens);
    let reserves: Vec<ReserveTokenDocument> = collect_all(collection).await?;
    Ok(reserves)
}

pub async fn find_all_users() -> Result<Vec<UserPositionDocument>, mongodb::error::Error> {
    let collection: Collection<UserPositionDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().user_positions);
    let users: Vec<UserPositionDocument> = collect_all(collection).await?;
    Ok(users)
}

pub async fn find_reserve_for_debt_token(
    token: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(token, ReserveTokenField::VariableDebtToken).await
}

pub async fn find_reserve_for_a_token(
    token: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(token, ReserveTokenField::AToken).await
}

pub async fn find_reserve_for_reserve_address(
    token: &str,
) -> Result<Option<ReserveTokenDocument>, mongodb::error::Error> {
    find_reserve_for_token(token, ReserveTokenField::Reserve).await
}

pub async fn find_reserve_for_token(
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

    find_one_as_option(collection, filter).await
}

pub async fn find_all_user_addresses() -> Vec<String> {
    let users = find_all_users().await.unwrap_or_else(|e| {
        eprintln!("Failed to find all users: {}", e);
        vec![]
    });

    let user_addresses: Vec<String> = users.iter().map(|u| u.userAddress.clone()).collect();

    // dbg!(&user_addresses);
    user_addresses
}

pub async fn find_all_block_numbers_from_solver_volume() -> Vec<u64> {
    let solver_volume = get_solver_volume().await.unwrap_or_else(|e| {
        eprintln!("Failed to get solver volume: {}", e);
        vec![]
    });

    let block_numbers: Vec<u64> = solver_volume.iter().map(|s| s.blockNumber).collect();

    // dbg!(&block_numbers);
    block_numbers
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

pub async fn get_user_position(
    user_address: &str,
) -> Result<UserPositionDocument, mongodb::error::Error> {
    let collection: Collection<UserPositionDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().user_positions);

    let filter = doc! { "userAddress": user_address };
    find_one(collection, filter).await
}

pub async fn find_user_events(
    user_address: &str,
) -> Result<Vec<MoneyMarketEventDocument>, mongodb::error::Error> {
    let collection: Collection<MoneyMarketEventDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().money_market_events);

    let filter = doc! { "$or": [
        { "user": user_address },
        { "from": user_address },
        { "to": user_address },
        { "onBehalfOf": user_address },
        { "repayer": user_address },
        { "target": user_address }
    ]};
    collect_all_with_filter(collection, filter).await
}

pub async fn find_token_events(
    token_address: &str,
) -> Result<Vec<MoneyMarketEventDocument>, mongodb::error::Error> {
    let collection: Collection<MoneyMarketEventDocument> = get_db()
        .await
        .database()
        .collection(get_collections_config().money_market_events);

    let reserve_from_a_token = find_reserve_for_a_token(token_address).await?;
    let reserve_from_debt_token = find_reserve_for_debt_token(token_address).await?;

    let reserve_address = match reserve_from_a_token {
        Some(reserve) => reserve.reserveAddress,
        None => match reserve_from_debt_token {
            Some(reserve) => reserve.reserveAddress,
            None => {
                eprintln!("No reserve found for token address: {}", token_address);
                std::process::exit(1) // No matching reserve found
            }
        },
    };

    // parse to valid eip55 Address
    let token_address_eip: Address =
        Address::from_str(&reserve_address).expect("Invalid token address format");
    let checksummed = token_address_eip.to_checksum(None);

    // Create filter to match either tokenAddress or reserve
    let filter = doc! { "$or": [
        { "tokenAddress": token_address },
        { "reserve": checksummed },
    ]};

    collect_all_with_filter(collection, filter).await
}

// GENERICS
//
async fn find_one<T>(
    collection: Collection<T>,
    filter: Document,
) -> Result<T, mongodb::error::Error>
where
    T: serde::de::DeserializeOwned + std::marker::Send + std::marker::Sync,
{
    match collection.find_one(filter).await {
        Ok(doc) => match doc {
            Some(document) => Ok(document),
            None => Err(mongodb::error::Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Document not found",
            ))),
        },
        Err(e) => {
            eprintln!("Error finding document: {}", e);
            Err(e)
        }
    }
}

async fn find_one_as_option<T>(
    collection: Collection<T>,
    filter: Document,
) -> Result<Option<T>, mongodb::error::Error>
where
    T: serde::de::DeserializeOwned + std::marker::Send + std::marker::Sync,
{
    match collection.find_one(filter).await {
        Ok(doc) => Ok(doc),
        Err(e) => {
            eprintln!("Error finding document: {}", e);
            Err(e)
        }
    }
}

async fn collect_all<T>(collection: Collection<T>) -> Result<Vec<T>, mongodb::error::Error>
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::marker::Send + std::marker::Sync,
{
    collect_all_with_filter(collection, doc! {}).await
}

async fn collect_all_with_filter<T>(
    collection: Collection<T>,
    filter: Document,
) -> Result<Vec<T>, mongodb::error::Error>
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::marker::Send + std::marker::Sync,
{
    collect_helper(collection, filter, None).await
}

async fn collect_helper<T>(
    collection: Collection<T>,
    search_filter: Document,
    some_return_filter: Option<Document>,
) -> Result<Vec<T>, mongodb::error::Error>
where
    T: serde::de::DeserializeOwned + serde::Serialize + std::marker::Send + std::marker::Sync,
{
    let mut docs: Vec<T> = vec![];
    let apply_return_filter = some_return_filter.is_some();
    let mut cursor = if apply_return_filter {
        let return_filter = some_return_filter.unwrap();
        let find_options = FindOptions::builder().projection(return_filter).build();
        collection
            .find(search_filter)
            .with_options(find_options)
            .await?
    } else {
        collection.find(search_filter).await?
    };

    while let Some(doc_result) = cursor.next().await {
        match doc_result {
            Ok(doc) => docs.push(doc),
            Err(e) => {
                eprintln!("Error collecting documents with filter. {}", e);
                return Err(e);
            }
        };
    }
    Ok(docs)
}
