use sodax_backend_analizer::{db, parse_args, Flag, HELP_MESSAGE};

fn main() {
    match parse_args() {
        Ok(flag) => match flag {
            Flag::Help => println!("{}", HELP_MESSAGE),
            Flag::AllTokens => match db::find_all_reserves() {
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
            Flag::ReserveToken(address) => match db::get_reserve_data_for_reserve_token(&address) {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for address: {}", address),
                Err(e) => eprintln!("Error: {}", e),
            },
            Flag::AToken(address) => match db::get_reserve_data_for_a_token(&address) {
                Ok(Some(reserve)) => println!("Reserve: {:#?}", reserve),
                Ok(None) => println!("No reserve found for aToken: {}", address),
                Err(e) => eprintln!("Error: {}", e),
            },
            Flag::VariableToken(address) => {
                match db::get_reserve_data_for_variable_debt_token(&address) {
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
