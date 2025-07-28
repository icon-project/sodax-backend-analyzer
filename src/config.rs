use dotenv::dotenv;
use std::env;

#[derive(Debug)]
pub struct Config {
    pub mongo_user: String,
    pub mongo_password: String,
    pub mongo_host: String,
    pub mongo_port: u32,
    pub mongo_db: String,
}

impl Config {
    pub fn new() -> Self {
        dotenv().ok();

        Config {
            mongo_user: env::var("MONGO_USER").expect("MONGO_USER must be set"),
            mongo_password: env::var("MONGO_PASSWORD").expect("MONGO_PASSWORD must be set"),
            mongo_host: env::var("MONGO_HOST").expect("MONGO_HOST must be set"),
            mongo_port: env::var("MONGO_PORT")
                .expect("MONGO_PORT must be set")
                .parse()
                .expect("MONGO_PORT must be a valid number"),
            mongo_db: env::var("MONGO_DB").expect("MONGO_DB must be set"),
        }
    }

    pub fn connection_string(&self) -> String {
        format!(
            "mongodb://{}:{}@{}:{}",
            self.mongo_user, self.mongo_password, self.mongo_host, self.mongo_port
        )
    }

    pub fn database_name(&self) -> String {
        self.mongo_db.clone()
    }
}

pub fn get_config() -> Config {
    // println!("Loading configuration...");
    // println!(
    //     "Using MongoDB connection string: {}",
    //     Config::new().connection_string()
    // );
    Config::new()
}
