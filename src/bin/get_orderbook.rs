use sodax_backend_analizer::db::get_orderbook;

fn main() {
    let orderbook = match get_orderbook() {
        Ok(docs) => docs,
        Err(e) => {
            eprintln!("Error retrieving orderbook: {}", e);
            vec![]
        }
    };

    dbg!(&orderbook[..2]);
}
