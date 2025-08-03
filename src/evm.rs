use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    sol,
};

sol! {
    #[sol(rpc)]
    contract ERC20 {
        function balanceOf(address owner) public view returns (uint256);

        function totalSupply() public view returns (uint256);
    }
}

sol! {
    #[derive(Debug)]
    struct ReserveConfigurationMap {
        uint256 data;
    }

    #[derive(Debug)]
    struct ReserveDataLegacy {
        ReserveConfigurationMap configuration;
        uint128 liquidityIndex;
        uint128 currentLiquidityRate;
        uint128 variableBorrowIndex;
        uint128 currentVariableBorrowRate;
        uint128 currentStableBorrowRate;
        uint40 lastUpdateTimestamp;
        uint16 id;
        address aTokenAddress;
        address stableDebtTokenAddress;
        address variableDebtTokenAddress;
        address interestRateStrategyAddress;
        uint128 accruedToTreasury;
        uint128 unbacked;
        uint128 isolationModeTotalDebt;
    }

    #[sol(rpc)]
    contract Pool {
        function getReserveData(address asset) public view returns (ReserveDataLegacy);
    }
}

const POOL_ADDRESS: &str = "0x553434896d39f867761859d0fe7189d2af70514e";

async fn get_provider() -> Result<impl Provider, Box<dyn std::error::Error>> {
    let provider = ProviderBuilder::new()
        .connect("https://rpc.soniclabs.com")
        .await?;
    Ok(provider)
}

pub async fn get_balance_of(
    token_address: &str,
    owner_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let provider = get_provider().await?;
    let token_address = token_address.parse::<Address>()?;
    let owner_address = owner_address.parse::<Address>()?;

    let contract = ERC20::new(token_address, provider);
    match contract.balanceOf(owner_address).call().await {
        Ok(balance) => Ok(u128::try_from(balance).unwrap_or(0)),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn get_total_supply(token_address: &str) -> Result<u128, Box<dyn std::error::Error>> {
    let provider = get_provider().await.unwrap();
    let token_address = token_address.parse::<Address>()?;
    let contract = ERC20::new(token_address, provider);
    match contract.totalSupply().call().await {
        Ok(total_supply) => Ok(u128::try_from(total_supply).unwrap_or(0)),
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

pub async fn get_reserve_data(
    asset_address: &str,
) -> Result<ReserveDataLegacy, Box<dyn std::error::Error>> {
    let provider = get_provider().await?;
    let asset_address = asset_address.parse::<Address>()?;

    let contract = Pool::new(POOL_ADDRESS.parse::<Address>()?, provider);
    match contract.getReserveData(asset_address).call().await {
        Ok(reserve_data) => {
            // Validate that the reserve data is not empty/default
            if reserve_data.liquidityIndex == 0
                && reserve_data.variableBorrowIndex == 0
                && reserve_data.aTokenAddress == Address::ZERO
            {
                return Err("Invalid asset address: reserve not found in pool".into());
            }
            Ok(reserve_data)
        }
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn get_atoken_liquidity_index(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let reserve_data = get_reserve_data(reserve_address).await?;
    Ok(reserve_data.liquidityIndex)
}

pub async fn get_variable_borrow_index(
    reserve_address: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let reserve_data = get_reserve_data(reserve_address).await?;
    Ok(reserve_data.variableBorrowIndex)
}
