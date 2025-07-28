use sodax_backend_analizer::db::get_collections;

fn main() {
    let collections = get_collections();
    println!("Collections: {:?}", collections);
}
