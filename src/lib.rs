pub mod config;
pub mod db;
pub mod evm;
pub mod models;

pub const HELP_MESSAGE: &str = r#"
sodax-backend-analizer - A CLI tool for analyzing database data for the SODAX backend

USAGE:
    sodax-backend-analizer [OPTIONS]

OPTIONS:
    --help                  Show this message
    --reserve-token <TOKEN_ADDRESS>  Returns the reserve token data for the given reserve token address
    --a-token <TOKEN_ADDRESS>         Returns the reserve token data for the given aToken address
    --variable-token <TOKEN_ADDRESS>  Returns the reserve token data for the given variable token
    address
    --user-position <WALLET_ADDRESS>  Returns the user position data for the given wallet address

EXAMPLES:
    sodax-backend-analizer --help
    sodax-backend-analizer --reserve-token <TOKEN_ADDRESS>
    sodax-backend-analizer --a-token <TOKEN_ADDRESS>
    sodax-backend-analizer --variable-token <TOKEN_ADDRESS>
    sodax-backend-analizer --user-position <WALLET_ADDRESS>
"#;

use std::env;

#[allow(dead_code)]
pub enum Flag {
    Help,
    ReserveToken(String),
    AToken(String),
    VariableToken(String),
    UserPosition(String),
    AllTokens,
}

pub fn parse_args() -> Result<Flag, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("Not enough arguments".to_string());
    }

    match args[1].as_str() {
        "--help" => Ok(Flag::Help),
        "--reserve-token" => {
            validate_arg_len(args.len()).unwrap();
            Ok(Flag::ReserveToken(args[2].clone()))
        }
        "--a-token" => {
            validate_arg_len(args.len()).unwrap();
            Ok(Flag::AToken(args[2].clone()))
        }
        "--variable-token" => {
            validate_arg_len(args.len()).unwrap();
            Ok(Flag::VariableToken(args[2].clone()))
        }
        "--user-position" => {
            validate_arg_len(args.len()).unwrap();
            Ok(Flag::UserPosition(args[2].clone()))
        }
        "--all-tokens" => Ok(Flag::AllTokens),
        _ => Err(format!("Unknown argument: {}", args[1])),
    }
}

#[allow(dead_code)]
fn validate_arg_len(len: usize) -> Result<(), Box<dyn std::error::Error>> {
    if len < 3 {
        return Err("Missing arguments".to_string().into());
    }

    Ok(())
}
