use alloy::{
    primitives::{Address, Uint},
    providers::{Provider, ProviderBuilder},
    sol,
};

sol! {
    #[sol(rpc)]
    contract ERC20 {
        function balanceOf(address owner) public view returns (uint256);
    }
}

async fn get_provider() -> Result<impl Provider, Box<dyn std::error::Error>> {
    let provider = ProviderBuilder::new()
        .connect("https://rpc.soniclabs.com")
        .await?;
    Ok(provider)
}

pub async fn get_balance_of(
    token_address: &str,
    owner_address: &str,
) -> Result<Uint<256, 4>, Box<dyn std::error::Error>> {
    let provider = get_provider().await?;
    let token_address = token_address.parse::<Address>()?;
    let owner_address = owner_address.parse::<Address>()?;

    let contract = ERC20::new(token_address, provider);
    match contract.balanceOf(owner_address).call().await {
        Ok(balance) => Ok(balance),
        Err(e) => Err(Box::new(e)),
    }
}
pub async fn get_last_block() -> Result<u64, Box<dyn std::error::Error>> {
    let provider = get_provider().await?;

    match provider.get_block_number().await {
        Ok(block_number) => Ok(block_number),
        Err(e) => Err(Box::new(e)),
    }
}
