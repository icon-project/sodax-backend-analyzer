use sodax_backend_analizer::{
    db::{
        find_all_reserves, get_reserve_data_for_a_token, get_reserve_data_for_reserve_token,
        get_reserve_data_for_variable_debt_token,
    },
    parse_args, Flag, HELP_MESSAGE,
};

#[tokio::main]
async fn main() {
    match parse_args() {
        Ok(flag) => match flag {
            Flag::Help => println!("{}", HELP_MESSAGE),
            Flag::AllTokens => match find_all_reserves().await {
                Ok(tokens) => {
                    if tokens.is_empty() {
                        println!("No tokens found.");
                    } else {
                        for token in tokens {
                            println!("Token: {:#?}", token);
                        }
                    }
                }
                Err(e) => eprintln!("Error fetching tokens: {}", e),
            },
            Flag::ReserveToken(address) => {
                match get_reserve_data_for_reserve_token(&address).await {
                    Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                    Ok(None) => println!("No reserve found for address: {}", address),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Flag::AToken(address) => match get_reserve_data_for_a_token(&address).await {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for aToken: {}", address),
                Err(e) => eprintln!("Error: {}", e),
            },
            Flag::VariableToken(address) => {
                match get_reserve_data_for_variable_debt_token(&address).await {
                    Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                    Ok(None) => println!("No reserve found for variable token: {}", address),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Flag::UserPosition(address) => {
                println!("User position for {} - not implemented yet", address);
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
