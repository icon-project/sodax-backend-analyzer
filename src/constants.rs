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
    --timestamp-coverage    Show the coverage percentage of documents with non-null timestamps
    --validate-timestamps [COUNT]  Validate database timestamps against on-chain block timestamps (optional: specify number of entries 1-100, defaults to all)
    --get-all-users         Print all user addresses from the database
    --get-all-reserves      Print all reserve token addresses and symbols
    --get-all-a-token       Print all aToken addresses and symbols
    --get-all-debt-token    Print all debt token addresses and symbols
    --reserve-token <TOKEN_ADDRESS>  Returns the reserve token data for the given reserve token address
    --a-token <TOKEN_ADDRESS>         Returns the reserve token data for the given aToken address
    --debt-token <TOKEN_ADDRESS>      Returns the reserve token data for the given debt token address
    --user-position <WALLET_ADDRESS>  Returns the user position data for the given wallet address
    --balance-of <USER_ADDRESS>       Get token balance for a user (requires one of: --reserve-token, --a-token, or --debt-token)
    --block <BLOCK_NUMBER>            Optional: specify a block number to query balance at that block (use with --balance-of)
    --get-token-events <TOKEN_ADDRESS> Get events for a specific token (reserve, aToken, or debt token)
    --get-user-events <USER_ADDRESS>  Get events for a specific user
    --calculate-from-events <USER_ADDRESS>  Calculate user's token balance from events (requires one of: --reserve-token, --a-token, or --debt-token)
    --calculate-from-events-reserve <RESERVE_ADDRESS>  Calculate token balances from events for every user in a reserve (compact table by default)
    --a-token-only                          Restrict --calculate-from-events-reserve to supply (aToken) side only
    --debt-token-only                       Restrict --calculate-from-events-reserve to variable-debt side only
    --verbose                               With --calculate-from-events-reserve, print the full per-user replay block instead of the compact table
    --inspect-user-position <USER_ADDRESS>  Inspect detailed user position for a specific token (requires either --a-token or --debt-token)
    --scaled                 Use scaled balances instead of real balances for validation (adds to validation flags)
    --no-report              Disable automatic report file generation (reports are saved to reports/ by default)

INDIVIDUAL VALIDATION OPTIONS:
    --validate-user-supply <USER_ADDRESS>  Validate user's aToken supply balance (requires --reserve-token)
    --validate-user-borrow <USER_ADDRESS>  Validate user's debt token balance (requires --reserve-token)
    --validate-token-supply               Validate total aToken supply for a reserve (requires --reserve-token)
    --validate-token-borrow              Validate total debt token supply for a reserve (requires --reserve-token)
    --validate-reserve-indexes <RESERVE_ADDRESS> Validate liquidity and borrow indexes for a specific reserve

BULK VALIDATION OPTIONS:
    --validate-user-all <USER_ADDRESS>    Validate all positions for a specific user
    --validate-users-all                  Validate all positions for all users
    --validate-token-all                 Validate all reserves in the marketplace
    --validate-all                       Validate everything (all reserves + all users)
    --validate-all-reserve-indexes       Validate indexes for all reserves
    --validate-from-events <USER_ADDRESS> Validate user positions by replaying raw events (3-way: events vs DB vs on-chain)
    --validate-from-events-all           Validate all users by replaying raw events
    --validate-partner-asset             Recompute partner_asset aggregates from solver_volume and report drift
    --partner <ADDRESS>                  Optional: limit --validate-partner-asset to a single receiver address
    --json                               Optional: emit JSON output (valid with --validate-partner-asset or --calculate-from-events-reserve)
    --threshold <PCT>                    Optional: rows within ±PCT of 1.0 are suppressed from the --validate-partner-asset table (default 0.0001)

SCALED VALIDATION:
    The --scaled flag can be combined with validation flags to compare scaled balances instead of real balances:
    - Scaled balances are the raw values stored in the database before applying liquidity/borrow indices
    - Real balances are calculated by applying the current liquidity/borrow indices to scaled balances
    - Use --scaled when you want to validate the raw database values against on-chain scaled balances

RESTRICTIONS:
    - You cannot combine --last-block, --help, --all-tokens, --orderbook, --timestamp-coverage, --validate-timestamps, --get-all-users, --get-all-reserves, --get-all-a-token, --get-all-debt-token, --validate-users-all, --validate-token-all, --validate-all, or --validate-all-reserve-indexes with other flags
    - You cannot combine --reserve-token, --a-token, and --debt-token together
    - --balance-of requires exactly one token type flag (--reserve-token, --a-token, or --debt-token)
    - --block can only be used with --balance-of
    - --inspect-user-position requires either --a-token or --debt-token to be specified
    - Individual validation flags require --reserve-token to be specified
    - --validate-user-all can be combined with --reserve-token for specific reserve validation
    - --scaled can only be combined with validation flags

EXAMPLES:
    # Basic operations
    sodax-backend-analizer --help
    sodax-backend-analizer --all-tokens
    sodax-backend-analizer --last-block
    sodax-backend-analizer --orderbook
    sodax-backend-analizer --timestamp-coverage
    sodax-backend-analizer --validate-timestamps
    sodax-backend-analizer --validate-timestamps 50
    sodax-backend-analizer --get-all-users
    sodax-backend-analizer --get-all-reserves
    sodax-backend-analizer --get-all-a-token
    sodax-backend-analizer --get-all-debt-token
    sodax-backend-analizer --reserve-token 0x1234567890abcdef...
    sodax-backend-analizer --a-token 0x1234567890abcdef...
    sodax-backend-analizer --debt-token 0x1234567890abcdef...
    sodax-backend-analizer --user-position 0x1234567890abcdef...
    sodax-backend-analizer --balance-of 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --balance-of 0xuser123... --a-token 0xtoken456... --block 12345678
    sodax-backend-analizer --get-token-events 0x1234567890abcdef...
    sodax-backend-analizer --get-user-events 0xuser123...
    sodax-backend-analizer --validate-reserve-indexes 0x1234567890abcdef...
    sodax-backend-analizer --validate-all-reserve-indexes
    sodax-backend-analizer --calculate-from-events 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --calculate-from-events 0xuser123... --a-token 0xatoken456...
    sodax-backend-analizer --calculate-from-events 0xuser123... --debt-token 0xdebt456...
    sodax-backend-analizer --inspect-user-position 0xuser123... --a-token 0xatoken456...
    sodax-backend-analizer --inspect-user-position 0xuser123... --debt-token 0xdebt456...

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

    # Event replay validation (3-way: events vs DB vs on-chain)
    sodax-backend-analizer --validate-from-events 0xuser123...
    sodax-backend-analizer --validate-from-events 0xuser123... --reserve-token 0xtoken456...
    sodax-backend-analizer --validate-from-events-all

    # Reserve-level event replay (every user in a reserve)
    sodax-backend-analizer --calculate-from-events-reserve 0xreserve...
    sodax-backend-analizer --calculate-from-events-reserve 0xreserve... --a-token-only
    sodax-backend-analizer --calculate-from-events-reserve 0xreserve... --debt-token-only
    sodax-backend-analizer --calculate-from-events-reserve 0xreserve... --verbose
    sodax-backend-analizer --calculate-from-events-reserve 0xreserve... --json

    # Partner asset drift validation (recompute from solver_volume)
    sodax-backend-analizer --validate-partner-asset
    sodax-backend-analizer --validate-partner-asset --partner 0xpartner123...
    sodax-backend-analizer --validate-partner-asset --threshold 0
    sodax-backend-analizer --validate-partner-asset --json

REPORT FILES:
    By default, every command (except --help) saves its output to a report file
    in the reports/ directory. The file is named report_<unix_timestamp>.txt.
    Use --no-report to disable this behavior.

OUTPUT FORMAT:
    Validation results show:
    - Database amount vs On-chain amount
    - Difference and percentage
    - Error messages for failed validations
    - Summary statistics for bulk operations
    - When using --scaled, amounts are shown as scaled balances (before index application)
"#;
