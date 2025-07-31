use sodax_backend_analizer::{
    db::{
        find_all_reserves, get_orderbook, get_reserve_data_for_a_token,
        get_reserve_data_for_reserve_token, get_reserve_data_for_variable_debt_token,
        get_user_position,
    },
    evm::{get_balance_of, get_last_block},
    parse_args, Flag, HELP_MESSAGE,
};

#[tokio::main]
async fn main() {
    let flags = match parse_args() {
        Ok(flags) => flags,
        Err(e) => {
            eprintln!("Error parsing flags: {}", e);
            std::process::exit(1);
        }
    };

    // if the --help flag is present, print the help message and exit
    if flags.iter().any(|f: &Flag| matches!(f, Flag::Help)) {
        println!("{}", HELP_MESSAGE)

    // if --orderbook was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::Orderbook)) {
        let book = match get_orderbook().await {
            Ok(book) => book,
            Err(e) => {
                eprintln!("Error fetching orderbook: {}", e);
                std::process::exit(1);
            }
        };
        if book.is_empty() {
            println!("No orderbook found.")
        } else {
            for order in book {
                println!("Order: {:#?}", order)
            }
        }

    // if --all-tokens was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::AllTokens)) {
        let tokens = match find_all_reserves().await {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("Error fetching tokens: {}", e);
                std::process::exit(1);
            }
        };
        if tokens.is_empty() {
            println!("No tokens found.")
        } else {
            for token in tokens {
                println!("Token: {:#?}", token)
            }
        }

    // if --last-block was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::LastBlock)) {
        match get_last_block().await {
            Ok(block) => {
                println!("Last block: {}", block)
            }
            Err(e) => {
                eprintln!("Error fetching last block: {}", e);
                std::process::exit(1);
            }
        };

    // if the --balance-of flag was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::BalanceOf(_))) {
        // when --balance-of is passed, we need to find the token address from which we will call
        // the balanceOf function
        let token_passed = flags
            .iter()
            .find_map(|f| match f {
                Flag::ReserveToken(address) => Some(address.clone()),
                Flag::AToken(address) => Some(address.clone()),
                Flag::VariableToken(address) => Some(address.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                eprintln!("Error: --balance-of requires a token address to be specified.");
                std::process::exit(1);
            });

        let user_address = flags
            .iter()
            .find_map(|f| match f {
                Flag::BalanceOf(address) => Some(address.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                eprintln!("Error: --balance-of requires a user address to be specified.");
                std::process::exit(1);
            });

        match get_balance_of(&token_passed, &user_address).await {
            Ok(balance) => println!(
                "Balance for user {} of token {}: {}",
                user_address, token_passed, balance
            ),
            Err(e) => {
                eprintln!("Error fetching balance: {}", e);
                std::process::exit(1);
            }
        }
    // if either --reserve-token, --a-token, or --variable-token was passed
    } else if flags.iter().any(|f: &Flag| {
        matches!(
            f,
            Flag::ReserveToken(_) | Flag::AToken(_) | Flag::VariableToken(_)
        )
    }) {
        //
        // Find the token address from the flags
        let token_address = flags
            .iter()
            .find_map(|f| match f {
                Flag::ReserveToken(address) => Some(("reserve", address.clone())),
                Flag::AToken(address) => Some(("a-token", address.clone())),
                Flag::VariableToken(address) => Some(("variable", address.clone())),
                _ => None,
            })
            .unwrap_or_else(|| {
                eprintln!("Error: You must specify a token address with --reserve-token, --a-token, or --variable-token.");
                std::process::exit(1);
            });

        match token_address.0 {
            "reserve" => match get_reserve_data_for_reserve_token(&token_address.1).await {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for address: {}", token_address.1),
                Err(e) => eprintln!("Error: {}", e),
            },
            "a-token" => match get_reserve_data_for_a_token(&token_address.1).await {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for aToken: {}", token_address.1),
                Err(e) => eprintln!("Error: {}", e),
            },
            "variable" => match get_reserve_data_for_variable_debt_token(&token_address.1).await {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for variable token: {}", token_address.1),
                Err(e) => eprintln!("Error: {}", e),
            },
            _ => {
                eprintln!("Error: Unknown token type.");
            }
        }
    // if --user-position was passed
    } else if flags.iter().any(|f: &Flag| matches!(f, Flag::UserPosition(_))) {
        let user_address = flags
            .iter()
            .find_map(|f| match f {
                Flag::UserPosition(address) => Some(address.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                eprintln!("Error: --user-position requires a user address to be specified.");
                std::process::exit(1);
            });

        match get_user_position(&user_address).await {
            Ok(Some(position)) => println!("User Position: {:#?}", position),
            Ok(None) => println!("No user position found for address: {}", user_address),
            Err(e) => {
                eprintln!("Error fetching user position: {}", e);
                std::process::exit(1);
            }
        }
    }
}
