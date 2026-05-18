use std::env;
use crate::structs::Flag;

pub fn parse_args() -> Result<Vec<Flag>, Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  let mut flags: Vec<Flag> = Vec::new();

  let mut i: usize = 1; // Start from 1 to skip the program name
  while i < args.len() {
    // Track whether this flag consumed the following argument
    let mut consumed_next_arg = false;
    let arg: &String = &args[i];
    match arg.as_str() {
      "--help" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::Help);
        break;
      }
      "--all-tokens" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::AllTokens);
        break;
      }
      "--last-block" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::LastBlock);
        break;
      }
      "--orderbook" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::Orderbook);
        break;
      }
      "--validate-users-all" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidateUsersAll);
      }
      "--validate-token-all" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidateTokenAll);
      }
      "--validate-all" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidateAll);
      }
      "--timestamp-coverage" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::TimestampCoverage);
        break;
      }
      "--validate-timestamps" => {
        // Optional argument: if next token is missing or a flag, treat as None
        if i + 1 >= args.len() || args[i + 1].starts_with("--") {
          flags.push(Flag::ValidateTimestamps(None));
          // no next arg consumed
          break;
        }
        flags.push(Flag::ValidateTimestamps(Some(args[i + 1].clone())));
        // No need to consume next arg because we break out of the loop
        break;
      }
      "--get-all-users" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::GetAllUsers);
        break;
      }
      "--get-all-reserves" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::GetAllReserves);
        break;
      }
      "--get-all-a-token" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::GetAllATokens);
        break;
      }
      "--get-all-debt-token" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::GetAllDebtTokens);
        break;
      }
      "--get-token-events" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::GetTokenEvents(args[i + 1].clone()));
        break;
      }
      "--get-user-events" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::GetUserEvents(args[i + 1].clone()));
        break;
      }
      "--validate-reserve-indexes" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ValidateReserveIndexes(args[i + 1].clone()));
        break;
      }
      "--calculate-from-events" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::CalculateFromEvents(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--calculate-from-events-reserve" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::CalculateFromEventsReserve(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--calculate-from-events-reserve-all" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::CalculateFromEventsReserveAll);
      }
      "--a-token-only" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ATokenOnly);
      }
      "--debt-token-only" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::DebtTokenOnly);
      }
      "--verbose" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::Verbose);
      }
      "--validate-all-reserve-indexes" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidateAllReserveIndexes);
        break;
      }
      "--validate-from-events" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ValidateFromEvents(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--validate-from-events-all" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidateFromEventsAll);
        break;
      }
      "--scaled" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::Scaled);
      }
      "--no-report" => {
        // Handled in main.rs, just skip here
      }
      "--validate-user-all" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ValidateUserAll(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--reserve-token" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ReserveToken(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--a-token" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::AToken(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--debt-token" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::DebtToken(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--user-position" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::UserPosition(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--balance-of" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::BalanceOf(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--validate-user-supply" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ValidateUserSupply(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--validate-user-borrow" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::ValidateUserBorrow(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--validate-token-supply" => {
        flags.push(Flag::ValidateTokenSupply);
      }
      "--validate-token-borrow" => {
        flags.push(Flag::ValidateTokenBorrow);
      }
      "--block" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        let block_number = args[i + 1].parse::<u64>().map_err(|_| {
          format!(
            "Invalid block number: {}. Must be a valid u64 integer.",
            args[i + 1]
          )
        })?;
        flags.push(Flag::Block(block_number));
        consumed_next_arg = true;
      }
      "--inspect-user-position" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::InspectUserPosition(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--validate-partner-asset" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::ValidatePartnerAsset);
      }
      "--partner" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        flags.push(Flag::Partner(args[i + 1].clone()));
        consumed_next_arg = true;
      }
      "--json" => {
        validate_flag_does_not_accept_argument(i, &args)?;
        flags.push(Flag::Json);
      }
      "--threshold" => {
        validate_flag_accepts_argument(i, args.len())?;
        validate_next_argument_is_not_flag(i, &args)?;
        let threshold = args[i + 1].parse::<f64>().map_err(|_| {
          format!(
            "Invalid threshold: {}. Must be a valid floating-point number.",
            args[i + 1]
          )
        })?;
        flags.push(Flag::Threshold(threshold));
        consumed_next_arg = true;
      }
      _ => return Err(format!("Unknown argument: {}", arg).into()),
    }
    // Move to the next argument
    // - For flags with arguments, increment by 2 (current arg + value)
    // - For flags without arguments, increment by 1 (current arg only)
    // - For optional-argument flags, increment based on whether the next arg was consumed
    if matches!(
      arg.as_str(),
      "--validate-token-supply" | "--validate-token-borrow"
    ) {
      i += 1;
    } else if consumed_next_arg {
      i += 2;
    } else {
      i += 1;
    }
  }

  // get boolean state of flags for comparison

  // boolean for --balance-of
  let has_balance_of = flags.iter().any(|flag| matches!(flag, Flag::BalanceOf(_)));

  // boolean for --user-position
  let has_user_position = flags
    .iter()
    .any(|flag| matches!(flag, Flag::UserPosition(_)));

  // boolean for --reserve-token
  let has_reserve_token = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ReserveToken(_)));

  // boolean for --a-token
  let has_a_token = flags.iter().any(|flag| matches!(flag, Flag::AToken(_)));

  // boolean for --debt-token
  let has_debt_token = flags.iter().any(|flag| matches!(flag, Flag::DebtToken(_)));

  // boolean for --validate-user-supply
  let has_validate_user_supply = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateUserSupply(_)));

  // boolean for --validate-user-borrow
  let has_validate_user_borrow = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateUserBorrow(_)));

  // boolean for --validate-token-supply
  let has_validate_token_supply = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateTokenSupply));

  // boolean for --validate-token-borrow
  let has_validate_token_borrow = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateTokenBorrow));

  // boolean for --scaled
  let has_scaled = flags.iter().any(|flag| matches!(flag, Flag::Scaled));

  // boolean for --calculate-from-events
  let has_calculate_from_events = flags
    .iter()
    .any(|flag| matches!(flag, Flag::CalculateFromEvents(_)));
  // boolean for --block
  let has_block = flags.iter().any(|flag| matches!(flag, Flag::Block(_)));

  // boolean for --inspect-user-position
  let has_inspect_user_position = flags
    .iter()
    .any(|flag| matches!(flag, Flag::InspectUserPosition(_)));

  // boolean for --validate-from-events
  let has_validate_from_events = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateFromEvents(_)));

  // boolean for --validate-from-events-all (used in combination validation below)
  let _has_validate_from_events_all = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidateFromEventsAll));

  // if no flags were added, add the help flag
  if flags.is_empty() {
    flags.push(Flag::Help);
    // return early with help HELP_MESSAGE
    return Ok(flags);
  }

  // the following flags cannot be combined with others
  //
  // --last-block
  // --help
  // --orderbook
  // --all-tokens
  // --validate-timestamps
  // --timestamp-coverage
  // --get-all-users
  // --get-all-reserves
  // --get-all-a-token
  // --get-all-debt-token
  // --validate-reserve-indexes
  // --validate-all-reserve-indexes
  if flags.iter().any(|flag| {
    (matches!(
      flag,
      Flag::LastBlock
        | Flag::Help
        | Flag::AllTokens
        | Flag::Orderbook
        | Flag::TimestampCoverage
        | Flag::GetAllUsers
        | Flag::GetAllReserves
        | Flag::GetAllATokens
        | Flag::GetAllDebtTokens
        | Flag::ValidateAllReserveIndexes
    ) && flags.len() > 1)
      || (matches!(
        flag,
        Flag::ValidateTimestamps(_)
          | Flag::GetTokenEvents(_)
          | Flag::GetUserEvents(_)
          | Flag::ValidateReserveIndexes(_)
      ) && flags.len() > 2)
  }) {
    return Err("You cannot combine --last-block, --help, --orderbook, --all-tokens, --validate-token-timestamp, --timestamp-coverage, --get-all-users, --get-all-reserves, --get-all-a-token, --get-all-debt-token, --validate-all-reserve-indexes with other flags. Use --help for more information.".into());
  }

  // the following flags can only be combined with --scaled
  //
  // --validate-users-all
  // --validate-user-all
  // --validate-token-all
  // --validate-all
  if flags.iter().any(|flag| {
    (matches!(
      flag,
      Flag::ValidateUsersAll | Flag::ValidateTokenAll | Flag::ValidateAll | Flag::ValidateFromEventsAll
    ) && flags.len() > 2)
      || (matches!(flag, Flag::ValidateUserAll(_)) && flags.len() > 3)
  }) {
    if !has_scaled {
      return Err("You can only combine --validate-users-all, --validate-user-all, --validate-token-all, --validate-all, --validate-from-events-all with --scaled. Use --help for more information.".into());
    }
    if flags.len() > 4 {
      return Err("You can only combine --validate-users-all, --validate-user-all, --validate-token-all, --validate-all, --validate-from-events-all with --scaled. Use --help for more information.".into());
    }
  }

  // --validate-from-events can be combined with --reserve-token (optional)
  if has_validate_from_events {
    let allowed_companions = flags.iter().all(|flag| {
      matches!(
        flag,
        Flag::ValidateFromEvents(_) | Flag::ReserveToken(_)
      )
    });
    if !allowed_companions {
      return Err("--validate-from-events can only be combined with --reserve-token. Use --help for more information.".into());
    }
  }

  // --validate-partner-asset can be combined with --partner, --json, --threshold (all optional)
  let has_validate_partner_asset = flags
    .iter()
    .any(|flag| matches!(flag, Flag::ValidatePartnerAsset));
  if has_validate_partner_asset {
    let allowed_companions = flags.iter().all(|flag| {
      matches!(
        flag,
        Flag::ValidatePartnerAsset | Flag::Partner(_) | Flag::Json | Flag::Threshold(_)
      )
    });
    if !allowed_companions {
      return Err("--validate-partner-asset can only be combined with --partner, --json, --threshold. Use --help for more information.".into());
    }
  }

  // --calculate-from-events-reserve can be combined with --a-token-only,
  // --debt-token-only (mutually exclusive), --verbose, --json (all optional)
  let has_calculate_from_events_reserve = flags
    .iter()
    .any(|flag| matches!(flag, Flag::CalculateFromEventsReserve(_)));
  let has_calculate_from_events_reserve_all = flags
    .iter()
    .any(|flag| matches!(flag, Flag::CalculateFromEventsReserveAll));
  let has_a_token_only = flags.iter().any(|flag| matches!(flag, Flag::ATokenOnly));
  let has_debt_token_only = flags.iter().any(|flag| matches!(flag, Flag::DebtTokenOnly));
  if has_calculate_from_events_reserve {
    let allowed_companions = flags.iter().all(|flag| {
      matches!(
        flag,
        Flag::CalculateFromEventsReserve(_)
          | Flag::ATokenOnly
          | Flag::DebtTokenOnly
          | Flag::Verbose
          | Flag::Json
      )
    });
    if !allowed_companions {
      return Err("--calculate-from-events-reserve can only be combined with --a-token-only, --debt-token-only, --verbose, --json. Use --help for more information.".into());
    }
    if has_a_token_only && has_debt_token_only {
      return Err("--a-token-only and --debt-token-only are mutually exclusive.".into());
    }
    // --verbose prints per-event detail; --json emits structured output. Combining them
    // would silently drop the verbose output, so reject the combination outright.
    let combo_verbose = flags.iter().any(|f| matches!(f, Flag::Verbose));
    let combo_json = flags.iter().any(|f| matches!(f, Flag::Json));
    if combo_verbose && combo_json {
      return Err("--verbose and --json cannot be combined.".into());
    }
  }

  // --calculate-from-events-reserve-all is the market-wide variant. It accepts the same
  // companions as --calculate-from-events-reserve except --verbose: running the verbose
  // per-event replay across every reserve in the market produces unusable amounts of
  // output, so verbose's value is restricted to single-reserve debugging.
  if has_calculate_from_events_reserve_all {
    let allowed_companions = flags.iter().all(|flag| {
      matches!(
        flag,
        Flag::CalculateFromEventsReserveAll | Flag::ATokenOnly | Flag::DebtTokenOnly | Flag::Json
      )
    });
    if !allowed_companions {
      return Err("--calculate-from-events-reserve-all can only be combined with --a-token-only, --debt-token-only, --json. Use --help for more information.".into());
    }
    if has_a_token_only && has_debt_token_only {
      return Err("--a-token-only and --debt-token-only are mutually exclusive.".into());
    }
  }

  // --partner and --threshold are only valid alongside --validate-partner-asset.
  // --json is valid with --validate-partner-asset OR either of the reserve-event-replay
  // flags. --a-token-only / --debt-token-only / --verbose pair only with the
  // reserve-event-replay flags (verbose with the single-reserve variant only).
  let has_partner_only_companion = flags
    .iter()
    .any(|flag| matches!(flag, Flag::Partner(_) | Flag::Threshold(_)));
  if has_partner_only_companion && !has_validate_partner_asset {
    return Err("--partner and --threshold can only be used with --validate-partner-asset.".into());
  }

  let has_json = flags.iter().any(|flag| matches!(flag, Flag::Json));
  if has_json
    && !has_validate_partner_asset
    && !has_calculate_from_events_reserve
    && !has_calculate_from_events_reserve_all
  {
    return Err("--json can only be used with --validate-partner-asset, --calculate-from-events-reserve, or --calculate-from-events-reserve-all.".into());
  }

  let has_verbose = flags.iter().any(|flag| matches!(flag, Flag::Verbose));
  if has_verbose && !has_calculate_from_events_reserve {
    return Err("--verbose can only be used with --calculate-from-events-reserve.".into());
  }
  if (has_a_token_only || has_debt_token_only)
    && !has_calculate_from_events_reserve
    && !has_calculate_from_events_reserve_all
  {
    return Err("--a-token-only and --debt-token-only can only be used with --calculate-from-events-reserve or --calculate-from-events-reserve-all.".into());
  }

  // the following flags need to be used acompanied by
  // --reserve-token:
  // --validate-user-supply
  // --validate-user-borrow
  // --validate-token-supply
  // --validate-token-borrow
  // --balance-of (can use aToken or variable token)
  // --user-position (can use aToken or variable token)
  // --calculate-from-events (can use aToken or variable token)
  if (has_balance_of
    || has_user_position
    || has_calculate_from_events
    || has_validate_user_supply
    || has_validate_user_borrow
    || has_validate_token_supply
    || has_validate_token_borrow)
    && flags.len() == 1
  {
    return Err("This flag cannot be used alone. Please specify a token address flag (--reserve-token, --a-token, or --debt-token)".into());
  }

  // if --balance-of, --user-position or --calculate-from-events is used, the user must provide
  // either reserve token, aToken, or variable token address
  if has_balance_of || has_user_position || has_calculate_from_events {
    let has_required_token = flags.iter().any(|flag| {
      matches!(
        flag,
        Flag::ReserveToken(_) | Flag::AToken(_) | Flag::DebtToken(_)
      )
    });
    if !has_required_token {
      return Err("You must provide a reserve token, aToken, or debt token address with --balance-of, --user-position, or --calculate-from-events".into());
    }
  }

  // if --block is used, it must be used with --balance-of
  if has_block && !has_balance_of {
    return Err("The --block flag can only be used with --balance-of".into());
  }

  // if any of the following is used:
  // --validate-user-supply
  // --validate-user-borrow
  // --validate-token-supply
  // --validate-token-borrow
  // --get-token-events
  //
  // the user must provide a --reserve-token flag
  if (has_validate_user_supply
    || has_validate_user_borrow
    || has_validate_token_supply
    || has_validate_token_borrow)
    && !has_reserve_token
  {
    return Err("You must use --reserve-token with --validate-user-supply, --validate-user-borrow, --validate-token-supply or --validate-token-borrow".into());
  }

  // cant combine --reserve-token, --a-token and --debt-token
  if (has_debt_token || has_a_token) && has_reserve_token || (has_a_token && has_debt_token) {
    return Err("You cannot combine --reserve-token, --a-token and --debt-token".into());
  }

  // if --inspect-user-position is used, it must be used with either --a-token or --debt-token
  if has_inspect_user_position && !has_a_token && !has_debt_token {
    return Err(
      "The --inspect-user-position flag requires either --a-token or --debt-token".into(),
    );
  }

  Ok(flags)
}

#[allow(dead_code)]
fn validate_flag_accepts_argument(i: usize, len: usize) -> Result<(), Box<dyn std::error::Error>> {
  if i >= len || i + 1 >= len {
    return Err("Missing arguments".to_string().into());
  }

  Ok(())
}

fn validate_flag_does_not_accept_argument(
  i: usize,
  args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
  if i + 1 < args.len() && !args[i + 1].starts_with("--") {
    return Err("This flag does not accept arguments".to_string().into());
  }
  Ok(())
}

fn validate_next_argument_is_not_flag(
  i: usize,
  args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
  if args[i + 1].starts_with("--") {
    return Err(format!("Expected an argument after '{}', but found a flag", args[i]).into());
  }
  Ok(())
}
