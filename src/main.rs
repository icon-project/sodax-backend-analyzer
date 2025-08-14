use sodax_backend_analizer::handlers::{
    handle_all_tokens, handle_help, handle_last_block, handle_orderbook, handle_balance_of,
    handle_user_position, handle_token, handle_validate_user_scaled_supply,
    handle_validate_user_scaled_borrow, handle_validate_token_scaled_supply,
    handle_validate_token_scaled_borrow, handle_validate_user_supply, handle_validate_user_borrow,
    handle_validate_token_supply, handle_validate_token_borrow, handle_validate_token_all,
    handle_validate_token_all_scaled, handle_validate_users_all, handle_validate_users_all_scaled,
    handle_validate_user_all, handle_validate_user_all_scaled, handle_validate_all,
    handle_validate_all_scaled, handle_timestamp_coverage, handle_validate_timestamp,
};
use sodax_backend_analizer::cli::parse_args;
use sodax_backend_analizer::structs::Flag;

#[tokio::main]
async fn main() {
    let flags = match parse_args() {
        Ok(flags) => flags,
        Err(e) => {
            eprintln!("Error parsing flags: {}", e);
            std::process::exit(1);
        }
    };

    // first handle the flags that can only be use
    // alone

    // if the --help flag is present, print the help message and exit
    if flags.iter().any(|f: &Flag| matches!(f, Flag::Help)) {
        handle_help().await;
        std::process::exit(0);

    // if --orderbook was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::Orderbook)) {
        handle_orderbook().await;
        std::process::exit(0);

    // if --all-tokens was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::AllTokens)) {
        handle_all_tokens().await;
        std::process::exit(0);

    // if --last-block was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::LastBlock)) {
        handle_last_block().await;
        std::process::exit(0);

    // if the --validate-token-all flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateTokenAll))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_token_all_scaled().await;
        } else {
            handle_validate_token_all().await;
        }
        std::process::exit(0);

    // if the --validate-users-all flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateUsersAll))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_users_all_scaled().await;
        } else {
            handle_validate_users_all().await;
        }
        std::process::exit(0);

    // if the --validate-user-all flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateUserAll(_)))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_user_all_scaled(flags).await;
        } else {
            handle_validate_user_all(flags).await;
        }
        std::process::exit(0);

    // if the --validate-all flag was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::ValidateAll)) {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_all_scaled().await;
        } else {
            handle_validate_all().await;
        }
        std::process::exit(0);

    // if the --timestamp-coverage flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::TimestampCoverage))
    {
        handle_timestamp_coverage().await;
        std::process::exit(0);

    // if the --validate-timestamps flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateTimestamps(_)))
    {
        handle_validate_timestamp(flags).await;
        std::process::exit(0);
    }

    // now handle the flags that can be used in
    // combination with others.
    // for this case we check the main flag that is
    // required to be used with a secondary flag. i.e:
    // --balance-of <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>
    // --balance-of <USER_ADDRES> --variable-token <token_address>
    // --balance-of <USER_ADDRESS> --a-token <TOKEN_ADDRESS>
    // --user-position <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>
    // --user-position <USER_ADDRESS> --a-token <TOKEN_ADDRESS>
    // --user-position <USER_ADDRESS> --variable-token <TOKEN_ADDRESS>
    // --validate-user-supply <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>
    // --validate-user-borrow <USER_ADDRESS> --reserve-token <TOKEN_ADDRESS>

    // if the --balance-of flag was passed
    if flags.iter().any(|f: &Flag| matches!(f, Flag::BalanceOf(_))) {
        handle_balance_of(flags).await;
        std::process::exit(0);

    // if the --user-position flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::UserPosition(_)))
    {
        handle_user_position(flags).await;
        std::process::exit(0);

    // if the --validate-user-supply [--scaled] flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateUserSupply(_)))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_user_scaled_supply(flags).await;
        } else {
            handle_validate_user_supply(flags).await;
        }
        std::process::exit(0);

    // if the --validate-user-borrow [--scaled] flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateUserBorrow(_)))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_user_scaled_borrow(flags).await;
        } else {
            handle_validate_user_borrow(flags).await;
        }
        std::process::exit(0);

    // if the --validate-token-supply [--scaled] flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateTokenSupply))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_token_scaled_supply(flags).await;
        } else {
            handle_validate_token_supply(flags).await;
        }
        std::process::exit(0);

    // if the --validate-token-borrow [--scaled] flag was passed
    } else if flags
        .iter()
        .any(|f: &Flag| matches!(f, Flag::ValidateTokenBorrow))
    {
        if flags.iter().any(|f: &Flag| matches!(f, Flag::Scaled)) {
            handle_validate_token_scaled_borrow(flags).await;
        } else {
            handle_validate_token_borrow(flags).await;
        }
        std::process::exit(0);
    }

    // NOTE: this should be the last check
    // if any of the token flags were passed
    // i.e:
    // --reserve-token <TOKEN_ADDRESS>
    // --a-token <TOKEN_ADDRESS>,
    // --variable-token <TOKEN_ADDRESS>
    if flags.iter().any(|f: &Flag| {
        matches!(
            f,
            Flag::ReserveToken(_) | Flag::AToken(_) | Flag::VariableToken(_)
        )
    }) {
        handle_token(flags).await;
        std::process::exit(0);
    }
}
