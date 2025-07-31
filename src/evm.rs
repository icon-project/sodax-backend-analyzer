use alloy::providers::{Provider, ProviderBuilder};

async fn get_provider() -> Result<impl Provider, Box<dyn std::error::Error>> {
    let provider = ProviderBuilder::new()
        .connect("https://rpc.soniclabs.com")
        .await?;
    Ok(provider)
}
pub async fn get_last_block() -> Result<u64, Box<dyn std::error::Error>> {
    let provider = get_provider().await?;

    match provider.get_block_number().await {
        Ok(block_number) => {
            println!("Last block number: {}", block_number);
            Ok(block_number)
        }
        Err(e) => {
            eprintln!("Failed to get the last block number: {}", e);
            Err(Box::new(e))
        }
    }
}
