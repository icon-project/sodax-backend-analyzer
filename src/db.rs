use crate::config::get_config;
use mongodb::{
    bson::{doc, Document},
    sync::{Client, Collection},
};

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

pub fn get_collections() -> Vec<String> {
    let mut collection_names: Vec<String> = vec![];
    let db = Database::new();
    let database = db.database();

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
