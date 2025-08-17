use std::env;
use crate::structs::Flag;

pub fn parse_args() -> Result<Vec<Flag>, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut flags: Vec<Flag> = Vec::new();

    if args.len() < 2 {
        return Err("Not enough arguments".into());
    }

    let mut i: usize = 1; // Start from 1 to skip the program name
    while i < args.len() {
        // Track whether this flag consumed the following argument
        let mut consumed_next_arg = false;
        let arg: &String = &args[i];
        match arg.as_str() {
            "--help" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::Help);
                break;
            }
            "--all-tokens" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::AllTokens);
                break;
            }
            "--last-block" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::LastBlock);
                break;
            }
            "--orderbook" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::Orderbook);
                break;
            }
            "--validate-users-all" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::ValidateUsersAll);
            }
            "--validate-token-all" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::ValidateTokenAll);
            }
            "--validate-all" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::ValidateAll);
            }
            "--timestamp-coverage" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::TimestampCoverage);
                break;
            }
            "--validate-timestamps" => {
                // Optional argument: if next token is missing or a flag, treat as None
                if i + 1 >= args.len() || args[i + 1].starts_with("--") {
                    flags.push(Flag::ValidateTimestamps(None));
                    // no next arg consumed
                    break;
                }
                flags.push(Flag::ValidateTimestamps(Some(args[i + 1].clone())));
                // No need to consume next arg because we break out of the loop
                break;
            }
            "--get-all-users" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::GetAllUsers);
                break;
            }
            "--get-all-reserves" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::GetAllReserves);
                break;
            }
            "--get-all-a-token" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::GetAllATokens);
                break;
            }
            "--get-all-debt-token" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::GetAllDebtTokens);
                break;
            }
            "--get-token-events" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::GetTokenEvents(args[i + 1].clone()));
                break;
            }
            "--get-user-events" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::GetUserEvents(args[i + 1].clone()));
                break;
            }
            "--validate-reserve-indexes" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::ValidateReserveIndexes(args[i + 1].clone()));
                break;
            }
            "--validate-all-reserve-indexes" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::ValidateAllReserveIndexes);
                break;
            }
            "--scaled" => {
                validate_flag_does_not_accept_argument(i, &args)?;
                flags.push(Flag::Scaled);
            }
            "--validate-user-all" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::ValidateUserAll(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--reserve-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::ReserveToken(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--a-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::AToken(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--debt-token" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::DebtToken(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--user-position" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::UserPosition(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--balance-of" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::BalanceOf(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--validate-user-supply" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::ValidateUserSupply(args[i + 1].clone()));
                consumed_next_arg = true;
            }
            "--validate-user-borrow" => {
                validate_flag_accepts_argument(i, args.len())?;
                validate_next_argument_is_not_flag(i, &args)?;
                flags.push(Flag::ValidateUserBorrow(args[i + 1].clone()));
                consumed_next_arg = true;
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
        // - For flags with arguments, increment by 2 (current arg + value)
        // - For flags without arguments, increment by 1 (current arg only)
        // - For optional-argument flags, increment based on whether the next arg was consumed
        if matches!(
            arg.as_str(),
            "--validate-token-supply" | "--validate-token-borrow"
        ) {
            i += 1;
        } else if consumed_next_arg {
            i += 2;
        } else {
            i += 1;
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

    // boolean for --debt-token
    let has_debt_token = flags.iter().any(|flag| matches!(flag, Flag::DebtToken(_)));

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

    // boolean for --scaled
    let has_scaled = flags.iter().any(|flag| matches!(flag, Flag::Scaled));

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
    // --validate-timestamps
    // --timestamp-coverage
    // --get-all-users
    // --get-all-reserves
    // --get-all-a-token
    // --get-all-debt-token
    // --validate-reserve-indexes
    // --validate-all-reserve-indexes
    if flags.iter().any(|flag| {
        (matches!(
            flag,
            Flag::LastBlock
                | Flag::Help
                | Flag::AllTokens
                | Flag::Orderbook
                | Flag::TimestampCoverage
                | Flag::GetAllUsers
                | Flag::GetAllReserves
                | Flag::GetAllATokens
                | Flag::GetAllDebtTokens
                | Flag::ValidateAllReserveIndexes
        ) && flags.len() > 1)
            || (matches!(
                flag,
                Flag::ValidateTimestamps(_)
                    | Flag::GetTokenEvents(_)
                    | Flag::GetUserEvents(_)
                    | Flag::ValidateReserveIndexes(_)
            ) && flags.len() > 2)
    }) {
        return Err("You cannot combine --last-block, --help, --orderbook, --all-tokens, --validate-token-timestamp, --timestamp-coverage, --get-all-users, --get-all-reserves, --get-all-a-token, --get-all-debt-token, --validate-all-reserve-indexes with other flags. Use --help for more information.".into());
    }

    // the following flags can only be combined with --scaled
    //
    // --validate-users-all
    // --validate-user-all
    // --validate-token-all
    // --validate-all
    if flags.iter().any(|flag| {
        (matches!(
            flag,
            Flag::ValidateUsersAll | Flag::ValidateTokenAll | Flag::ValidateAll
        ) && flags.len() > 2)
            || (matches!(flag, Flag::ValidateUserAll(_)) && flags.len() > 3)
    }) {
        if !has_scaled {
            return Err("You can only combine --validate-users-all, --validate-user-all, --validate-token-all, --validate-all with --scaled. Use --help for more information.".into());
        }
        if flags.len() > 4 {
            return Err("You can only combine --validate-users-all, --validate-user-all, --validate-token-all, --validate-all with --scaled. Use --help for more information.".into());
        }
    }

    // the following flags need to be used acompanied by
    // --reserve-token:
    // --validate-user-supply
    // --validate-user-borrow
    // --validate-token-supply
    // --validate-token-borrow
    // --balance-of (can use aToken or variable token)
    // --user-position (can use aToken or variable token)
    if (has_balance_of
        || has_user_position
        || has_validate_user_supply
        || has_validate_user_borrow
        || has_validate_token_supply
        || has_validate_token_borrow)
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
                Flag::ReserveToken(_) | Flag::AToken(_) | Flag::DebtToken(_)
            )
        });
        if !has_required_token {
            return Err("You must provide a reserve token, aToken, or debt token address with --balance-of or --user-position".into());
        }
    }

    // if any of the following is used:
    // --validate-user-supply
    // --validate-user-borrow
    // --validate-token-supply
    // --validate-token-borrow
    // --get-token-events
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

    // cant combine --reserve-token, --a-token and --debt-token
    if (has_debt_token || has_a_token) && has_reserve_token || (has_a_token && has_debt_token) {
        return Err("You cannot combine --reserve-token, --a-token and --debt-token".into());
    }

    Ok(flags)
}

#[allow(dead_code)]
fn validate_flag_accepts_argument(i: usize, len: usize) -> Result<(), Box<dyn std::error::Error>> {
    if i >= len || i + 1 >= len {
        return Err("Missing arguments".to_string().into());
    }

    Ok(())
}

fn validate_flag_does_not_accept_argument(
    i: usize,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if i + 1 < args.len() && !args[i + 1].starts_with("--") {
        return Err("This flag does not accept arguments".to_string().into());
    }
    Ok(())
}

fn validate_next_argument_is_not_flag(
    i: usize,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if args[i + 1].starts_with("--") {
        return Err(format!("Expected an argument after '{}', but found a flag", args[i]).into());
    }
    Ok(())
}
