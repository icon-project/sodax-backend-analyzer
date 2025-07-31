pub mod config;
pub mod db;
pub mod evm;
pub mod models;

pub const HELP_MESSAGE: &str = r#"
sodax-backend-analizer - A CLI tool for analyzing database data for the SODAX backend

USAGE:
    sodax-backend-analizer [OPTIONS]

OPTIONS:
    --help                  Show this help message
    --all-tokens            List all reserve tokens in the database
    --last-block            Get the latest block number from the blockchain
    --orderbook             Get all orderbook data from the database
    --reserve-token <TOKEN_ADDRESS>  Returns the reserve token data for the given reserve token address
    --a-token <TOKEN_ADDRESS>         Returns the reserve token data for the given aToken address
    --variable-token <TOKEN_ADDRESS>  Returns the reserve token data for the given variable token address
    --user-position <WALLET_ADDRESS>  Returns the user position data for the given wallet address
    --balance-of <USER_ADDRESS>       Get token balance for a user (requires one of: --reserve-token, --a-token, or --variable-token)

RESTRICTIONS:
    - You cannot combine --last-block, --help, --all-tokens, or --orderbook with other flags
    - You cannot combine --reserve-token, --a-token, and --variable-token together
    - --balance-of requires exactly one token type flag (--reserve-token, --a-token, or --variable-token)

EXAMPLES:
    sodax-backend-analizer --help
    sodax-backend-analizer --all-tokens
    sodax-backend-analizer --last-block
    sodax-backend-analizer --orderbook
    sodax-backend-analizer --reserve-token 0x1234567890abcdef...
    sodax-backend-analizer --a-token 0x1234567890abcdef...
    sodax-backend-analizer --variable-token 0x1234567890abcdef...
    sodax-backend-analizer --user-position 0x1234567890abcdef...
    sodax-backend-analizer --balance-of 0xuser123... --reserve-token 0xtoken456...
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
    BalanceOf(String),
    LastBlock,
    Orderbook,
}

pub fn parse_args() -> Result<Vec<Flag>, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut flags: Vec<Flag> = Vec::new();

    if args.len() < 2 {
        return Err("Not enough arguments".into());
    }

    let mut i: usize = 1; // Start from 1 to skip the program name
    while i < args.len() {
        let arg: &String = &args[i];
        match arg.as_str() {
            "--help" => {
                flags.push(Flag::Help);
                break;
            }
            "--all-tokens" => {
                flags.push(Flag::AllTokens);
                break;
            }
            "--last-block" => {
                flags.push(Flag::LastBlock);
                break;
            }
            "--orderbook" => {
                flags.push(Flag::Orderbook);
                break;
            }
            "--reserve-token" => {
                validate_arg_len(i, args.len())?;
                flags.push(Flag::ReserveToken(args[i + 1].clone()));
            }
            "--a-token" => {
                validate_arg_len(i, args.len())?;
                flags.push(Flag::AToken(args[i + 1].clone()));
            }
            "--variable-token" => {
                validate_arg_len(i, args.len())?;
                flags.push(Flag::VariableToken(args[i + 1].clone()));
            }
            "--user-position" => {
                validate_arg_len(i, args.len())?;
                flags.push(Flag::UserPosition(args[i + 1].clone()));
            }
            "--balance-of" => {
                validate_arg_len(i, args.len())?;
                flags.push(Flag::BalanceOf(args[i + 1].clone()));
            }
            _ => return Err(format!("Unknown argument: {}", arg).into()),
        }
        // Move to the next argument (current arg + value)
        i += 2;
    }

    // if no flags were added, add the help flag
    if flags.is_empty() {
        flags.push(Flag::Help);
    }

    // if --last-block, --help --orderbook or --all-tokens was passed, there should be no other flags
    if flags.iter().any(|flag| {
        matches!(
            flag,
            Flag::LastBlock | Flag::Help | Flag::AllTokens | Flag::Orderbook
        )
    }) && flags.len() > 1
    {
        return Err("You cannot combine --last-block, --help, --orderbook or --all-tokens with other flags. Use --help for more information.".into());
    }

    // detect if --balance-of is used without a token address
    let has_balance_of = flags.iter().any(|flag| matches!(flag, Flag::BalanceOf(_)));

    if has_balance_of && flags.len() == 1 {
        return Err("Missing token address for --balance-of".into());
    }

    // if --balance-of is used, the user must provide
    // either reserve token, aToken, or variable token address
    if has_balance_of {
        let has_required_token = flags.iter().any(|flag| {
            matches!(
                flag,
                Flag::ReserveToken(_) | Flag::AToken(_) | Flag::VariableToken(_)
            )
        });
        if !has_required_token {
            return Err("You must provide a reserve token, aToken, or variable token address with --balance-of".into());
        }
    }

    // cant combine --reserve-token, --a-token and --variable-token
    let has_reserve_token = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ReserveToken(_)));
    let has_a_token = flags.iter().any(|flag| matches!(flag, Flag::AToken(_)));
    let has_variable_token = flags
        .iter()
        .any(|flag| matches!(flag, Flag::VariableToken(_)));
    if (has_variable_token || has_a_token) && has_reserve_token
        || (has_a_token && has_variable_token)
    {
        return Err("You cannot combine --reserve-token, --a-token and --variable-token".into());
    }

    Ok(flags)
}

#[allow(dead_code)]
fn validate_arg_len(i: usize, len: usize) -> Result<(), Box<dyn std::error::Error>> {
    if i >= len {
        return Err("Missing arguments".to_string().into());
    }

    Ok(())
}
