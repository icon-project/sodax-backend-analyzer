use crate::constants::{RAY, HALF_RAY};
use crate::structs::{Flag, FlagType};
use primitive_types::U256;

// Returns an optional value for flags that may or may not carry a value (e.g., ValidateTimestamps)
pub fn extract_optional_value_from_flags(flags: &[Flag], flag_type: FlagType) -> Option<String> {
    flags.iter().find_map(|f| match (f, &flag_type) {
        (Flag::ReserveToken(value), FlagType::ReserveToken) => Some(value.clone()),
        (Flag::AToken(value), FlagType::AToken) => Some(value.clone()),
        (Flag::VariableToken(value), FlagType::VariableToken) => Some(value.clone()),
        (Flag::UserPosition(value), FlagType::UserPosition) => Some(value.clone()),
        (Flag::BalanceOf(value), FlagType::BalanceOf) => Some(value.clone()),
        (Flag::ValidateUserSupply(value), FlagType::ValidateUserSupply) => Some(value.clone()),
        (Flag::ValidateUserBorrow(value), FlagType::ValidateUserBorrow) => Some(value.clone()),
        (Flag::ValidateUserAll(value), FlagType::ValidateUserAll) => Some(value.clone()),
        (Flag::ValidateTimestamps(value_opt), FlagType::ValidateTimestamps) => value_opt.clone(),
        _ => None,
    })
}

// Convenience function for backward compatibility
pub fn extract_value_from_flags_or_exit(
    flags: Vec<Flag>,
    flag_type: FlagType,
    error_message: &str,
) -> String {
    match extract_optional_value_from_flags(&flags, flag_type) {
        Some(v) => v,
        None => {
            eprintln!("{}", error_message);
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    DivisionByZero,
    Overflow,
}

/// Multiplies two ray-scaled numbers with half-up rounding:
/// result = (a * b + HALF_RAY) / RAY
pub fn ray_mul(a: U256, b: U256) -> Result<U256, MathError> {
    let product = a.checked_mul(b).ok_or(MathError::Overflow)?;
    let with_round = product
        .checked_add(U256::from(HALF_RAY))
        .ok_or(MathError::Overflow)?;
    with_round
        .checked_div(U256::from(RAY))
        .ok_or(MathError::Overflow)
}

/// Divides two ray-scaled numbers with half-up rounding:
/// result = (a * RAY + b/2) / b
pub fn ray_div(a: U256, b: U256) -> Result<U256, MathError> {
    if b.is_zero() {
        return Err(MathError::DivisionByZero);
    }
    let scaled = a.checked_mul(U256::from(RAY)).ok_or(MathError::Overflow)?;
    let with_round = scaled
        .checked_add(b >> 1) // b/2
        .ok_or(MathError::Overflow)?;
    with_round.checked_div(b).ok_or(MathError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(n: u128) -> U256 {
        U256::from(n) * U256::from(RAY)
    }

    #[test]
    fn mul_identity() {
        // 1.0 * x = x
        let x = ray(1234);
        assert_eq!(ray_mul(ray(1), x).unwrap(), x);
    }

    #[test]
    fn div_identity() {
        // x / 1.0 = x
        let x = ray(987_654_321);
        assert_eq!(ray_div(x, ray(1)).unwrap(), x);
    }

    #[test]
    fn half_up_rounding_mul() {
        // Test that 1.0 * 1.5 = 1.5 (no rounding needed)
        let a = ray(1);
        let b = ray(3) / U256::from(2u8); // 1.5 ray
        assert_eq!(ray_mul(a, b).unwrap(), ray(3) / U256::from(2u8));
    }

    #[test]
    fn half_up_rounding_div() {
        // Test that 1.5 / 1.0 = 1.5 (no rounding needed)
        let a = ray(3) / U256::from(2u8); // 1.5 ray
        assert_eq!(ray_div(a, ray(1)).unwrap(), ray(3) / U256::from(2u8));
    }

    #[test]
    fn division_by_zero() {
        assert!(matches!(
            ray_div(ray(1), U256::zero()),
            Err(MathError::DivisionByZero)
        ));
    }
}
