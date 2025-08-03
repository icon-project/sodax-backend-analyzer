use std::env;

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

INDIVIDUAL VALIDATION OPTIONS:
    --validate-user-supply <USER_ADDRESS>  Validate user's aToken supply balance (requires --reserve-token)
    --validate-user-borrow <USER_ADDRESS>  Validate user's debt token balance (requires --reserve-token)
    --validate-token-supply               Validate total aToken supply for a reserve (requires --reserve-token)
    --validate-token-borrow              Validate total debt token supply for a reserve (requires --reserve-token)

BULK VALIDATION OPTIONS:
    --validate-user-all <USER_ADDRESS>    Validate all positions for a specific user
    --validate-users-all                  Validate all positions for all users
    --validate-token-all                 Validate all reserves in the marketplace
    --validate-all                       Validate everything (all reserves + all users)

RESTRICTIONS:
    - You cannot combine --last-block, --help, --all-tokens, --orderbook, --validate-users-all, --validate-token-all, or --validate-all with other flags
    - You cannot combine --reserve-token, --a-token, and --variable-token together
    - --balance-of requires exactly one token type flag (--reserve-token, --a-token, or --variable-token)
    - Individual validation flags require --reserve-token to be specified
    - --validate-user-all can be combined with --reserve-token for specific reserve validation

EXAMPLES:
    # Basic operations
    sodax-backend-analizer --help
    sodax-backend-analizer --all-tokens
    sodax-backend-analizer --last-block
    sodax-backend-analizer --orderbook
    sodax-backend-analizer --reserve-token 0x1234567890abcdef...
    sodax-backend-analizer --a-token 0x1234567890abcdef...
    sodax-backend-analizer --variable-token 0x1234567890abcdef...
    sodax-backend-analizer --user-position 0x1234567890abcdef...
    sodax-backend-analizer --balance-of 0xuser123... --reserve-token 0xtoken456...

    # Individual validation
    sodax-backend-analizer --validate-user-supply 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-user-borrow 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-token-supply --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-token-borrow --reserve-token 0xtoken456...

    # Bulk validation
    sodax-backend-analizer --validate-user-all 0xuser123...
    sodax-backend-analizer --validate-users-all
    sodax-backend-analizer --validate-token-all
    sodax-backend-analizer --validate-all

OUTPUT FORMAT:
    Validation results show:
    - Database amount vs On-chain amount
    - Difference and percentage
    - Error messages for failed validations
    - Summary statistics for bulk operations
"#;

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
    ValidateUserSupply(String),
    ValidateUserBorrow(String),
    ValidateTokenSupply,
    ValidateTokenBorrow,
    ValidateUserAll(String),
    ValidateUsersAll,
    ValidateTokenAll,
    ValidateAll,
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
            "--validate-users-all" => {
                flags.push(Flag::ValidateUsersAll);
                break;
            }
            "--validate-token-all" => {
                flags.push(Flag::ValidateTokenAll);
                break;
            }
            "--validate-all" => {
                flags.push(Flag::ValidateAll);
                break;
            }
            "--validate-user-all" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::ValidateUserAll(args[i + 1].clone()));
                break;
            }
            "--reserve-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::ReserveToken(args[i + 1].clone()));
            }
            "--a-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::AToken(args[i + 1].clone()));
            }
            "--variable-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::VariableToken(args[i + 1].clone()));
            }
            "--user-position" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::UserPosition(args[i + 1].clone()));
            }
            "--balance-of" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::BalanceOf(args[i + 1].clone()));
            }
            "--validate-user-supply" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::ValidateUserSupply(args[i + 1].clone()));
            }
            "--validate-user-borrow" => {
                validate_flag_accepts_argument(i, args.len())?;
                flags.push(Flag::ValidateUserBorrow(args[i + 1].clone()));
            }
            "--validate-token-supply" => {
                flags.push(Flag::ValidateTokenSupply);
            }
            "--validate-token-borrow" => {
                flags.push(Flag::ValidateTokenBorrow);
            }
            _ => return Err(format!("Unknown argument: {}", arg).into()),
        }
        // Move to the next argument
        // For flags with arguments, increment by 2 (current arg + value)
        // For flags without arguments, increment by 1 (current arg only)
        if matches!(
            arg.as_str(),
            "--validate-token-supply" | "--validate-token-borrow"
        ) {
            i += 1;
        } else {
            i += 2;
        }
    }

    // get boolean state of flags for comparison

    // boolean for --balance-of
    let has_balance_of = flags.iter().any(|flag| matches!(flag, Flag::BalanceOf(_)));

    // boolean for --user-position
    let has_user_position = flags
        .iter()
        .any(|flag| matches!(flag, Flag::UserPosition(_)));

    // boolean for --reserve-token
    let has_reserve_token = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ReserveToken(_)));

    // boolean for --a-token
    let has_a_token = flags.iter().any(|flag| matches!(flag, Flag::AToken(_)));

    // boolean for --variable-token
    let has_variable_token = flags
        .iter()
        .any(|flag| matches!(flag, Flag::VariableToken(_)));

    // boolean for --validate-user-supply
    let has_validate_user_supply = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ValidateUserSupply(_)));

    // boolean for --validate-user-borrow
    let has_validate_user_borrow = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ValidateUserBorrow(_)));

    // boolean for --validate-token-supply
    let has_validate_token_supply = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ValidateTokenSupply));

    // boolean for --validate-token-borrow
    let has_validate_token_borrow = flags
        .iter()
        .any(|flag| matches!(flag, Flag::ValidateTokenBorrow));

    // if no flags were added, add the help flag
    if flags.is_empty() {
        flags.push(Flag::Help);
        // return early with help HELP_MESSAGE
        return Ok(flags);
    }

    // the following flags cannot be combined with others
    //
    // --last-block
    // --help
    // --orderbook
    // --all-tokens
    // --validate-users-all
    // --validate-user-all
    // --validate-token-all
    // --validate-all
    if flags.iter().any(|flag| {
        (matches!(
            flag,
            Flag::LastBlock
                | Flag::Help
                | Flag::AllTokens
                | Flag::Orderbook
                | Flag::ValidateUsersAll
                | Flag::ValidateTokenAll
                | Flag::ValidateAll
        ) && flags.len() > 1)
                    || (matches!(
            flag,
            Flag::ValidateUserAll(_)
        ) && flags.len() > 2)
    }) {
        return Err("You cannot combine --last-block, --help, --orderbook, --all-tokens, --validate-[user|users|token]-all with other flags. Use --help for more information.".into());
    }

    // the following flags need to be used acompanied by
    // --reserve-token:
    // --validate-user-supply
    // --validate-user-borrow
    // --validate-token-supply
    // --validate-token-borrow
    // --balance-of (can use aToken or variable token)
    // --user-position (can use aToken or variable token)
    if (has_balance_of || has_user_position || has_validate_user_supply || has_validate_user_borrow)
        && flags.len() == 1
    {
        return Err("Missing --reserve-token address flag".into());
    }

    // if --balance-of or --user-position is used, the user must provide
    // either reserve token, aToken, or variable token address
    if has_balance_of || has_user_position {
        let has_required_token = flags.iter().any(|flag| {
            matches!(
                flag,
                Flag::ReserveToken(_) | Flag::AToken(_) | Flag::VariableToken(_)
            )
        });
        if !has_required_token {
            return Err("You must provide a reserve token, aToken, or variable token address with --balance-of or --user-position".into());
        }
    }

    // if any of the following is used:
    // --validate-user-supply
    // --validate-user-borrow
    // --validate-token-supply
    // --validate-token-borrow
    //
    // the user must provide a --reserve-token flag
    if (has_validate_user_supply
        || has_validate_user_borrow
        || has_validate_token_supply
        || has_validate_token_borrow)
        && !has_reserve_token
    {
        return Err("You must use --reserve-token with --validate-user-supply, --validate-user-borrow, --validate-token-supply or --validate-token-borrow".into());
    }

    // cant combine --reserve-token, --a-token and --variable-token
    if (has_variable_token || has_a_token) && has_reserve_token
        || (has_a_token && has_variable_token)
    {
        return Err("You cannot combine --reserve-token, --a-token and --variable-token".into());
    }

    Ok(flags)
}

#[allow(dead_code)]
fn validate_flag_accepts_argument(i: usize, len: usize) -> Result<(), Box<dyn std::error::Error>> {
    if i >= len {
        return Err("Missing arguments".to_string().into());
    }

    Ok(())
}

