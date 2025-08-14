// Aave constants
pub const RAY: u128 = 1_000_000_000_000_000_000_000_000_000; // 10^27
pub const HALF_RAY: u128 = 500_000_000_000_000_000_000_000_000; // 5e26 use std::env;
pub const HELP_MESSAGE: &str = r#"
sodax-backend-analizer - A CLI tool for analyzing database data for the SODAX backend

USAGE:
    sodax-backend-analizer [OPTIONS]

OPTIONS:
    --help                  Show this help message
    --all-tokens            List all reserve tokens in the database
    --last-block            Get the latest block number from the blockchain
    --orderbook             Get all orderbook data from the database
    --reserve-token <TOKEN_ADDRESS>  Returns the reserve token data for the given reserve token address
    --a-token <TOKEN_ADDRESS>         Returns the reserve token data for the given aToken address
    --variable-token <TOKEN_ADDRESS>  Returns the reserve token data for the given variable token address
    --user-position <WALLET_ADDRESS>  Returns the user position data for the given wallet address
    --balance-of <USER_ADDRESS>       Get token balance for a user (requires one of: --reserve-token, --a-token, or --variable-token)
    --scaled                 Use scaled balances instead of real balances for validation (adds to validation flags)

INDIVIDUAL VALIDATION OPTIONS:
    --validate-user-supply <USER_ADDRESS>  Validate user's aToken supply balance (requires --reserve-token)
    --validate-user-borrow <USER_ADDRESS>  Validate user's debt token balance (requires --reserve-token)
    --validate-token-supply               Validate total aToken supply for a reserve (requires --reserve-token)
    --validate-token-borrow              Validate total debt token supply for a reserve (requires --reserve-token)

BULK VALIDATION OPTIONS:
    --validate-user-all <USER_ADDRESS>    Validate all positions for a specific user
    --validate-users-all                  Validate all positions for all users
    --validate-token-all                 Validate all reserves in the marketplace
    --validate-all                       Validate everything (all reserves + all users)

SCALED VALIDATION:
    The --scaled flag can be combined with validation flags to compare scaled balances instead of real balances:
    - Scaled balances are the raw values stored in the database before applying liquidity/borrow indices
    - Real balances are calculated by applying the current liquidity/borrow indices to scaled balances
    - Use --scaled when you want to validate the raw database values against on-chain scaled balances

RESTRICTIONS:
    - You cannot combine --last-block, --help, --all-tokens, --orderbook, --validate-users-all, --validate-token-all, or --validate-all with other flags
    - You cannot combine --reserve-token, --a-token, and --variable-token together
    - --balance-of requires exactly one token type flag (--reserve-token, --a-token, or --variable-token)
    - Individual validation flags require --reserve-token to be specified
    - --validate-user-all can be combined with --reserve-token for specific reserve validation
    - --scaled can only be combined with validation flags

EXAMPLES:
    # Basic operations
    sodax-backend-analizer --help
    sodax-backend-analizer --all-tokens
    sodax-backend-analizer --last-block
    sodax-backend-analizer --orderbook
    sodax-backend-analizer --reserve-token 0x1234567890abcdef...
    sodax-backend-analizer --a-token 0x1234567890abcdef...
    sodax-backend-analizer --variable-token 0x1234567890abcdef...
    sodax-backend-analizer --user-position 0x1234567890abcdef...
    sodax-backend-analizer --balance-of 0xuser123... --reserve-token 0xtoken456...

    # Individual validation (real balances)
    sodax-backend-analizer --validate-user-supply 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-user-borrow 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-token-supply --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-token-borrow --reserve-token 0xtoken456...

    # Individual validation (scaled balances)
    sodax-backend-analizer --validate-user-supply 0xuser123... --reserve-token 0xtoken456... --scaled
    sodax-backend-analizer --validate-user-borrow 0xuser123... --reserve-token 0xtoken456... --scaled
    sodax-backend-analizer --validate-token-supply --reserve-token 0xtoken456... --scaled
    sodax-backend-analizer --validate-token-borrow --reserve-token 0xtoken456... --scaled

    # Bulk validation (real balances)
    sodax-backend-analizer --validate-user-all 0xuser123...
    sodax-backend-analizer --validate-users-all
    sodax-backend-analizer --validate-token-all
    sodax-backend-analizer --validate-all

    # Bulk validation (scaled balances)
    sodax-backend-analizer --validate-user-all 0xuser123... --scaled
    sodax-backend-analizer --validate-users-all --scaled
    sodax-backend-analizer --validate-token-all --scaled
    sodax-backend-analizer --validate-all --scaled

OUTPUT FORMAT:
    Validation results show:
    - Database amount vs On-chain amount
    - Difference and percentage
    - Error messages for failed validations
    - Summary statistics for bulk operations
    - When using --scaled, amounts are shown as scaled balances (before index application)
"#;
