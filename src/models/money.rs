use crate::errors::RowError;
use crate::structs::money::Money;
use std::fmt::{Display, Formatter, Result as FmtResult};

impl Money {
    const SCALE_U64: u64 = 10_000;
    const SCALE_U128: u128 = 10_000;

    pub fn parse(text: &str) -> Result<Money, RowError> {
        let mut units: i64 = 0;
        let mut frac_digits: usize = 0;
        let mut seen_dot = false;
        let mut seen_digit = false;
        for b in text.bytes() {
            if b == b'.' && !seen_dot {
                seen_dot = true;
                continue;
            }
            let Some(digit) = b.checked_sub(b'0') else {
                return Err(RowError::BadAmount);
            };
            if digit > 9 {
                return Err(RowError::BadAmount);
            }
            if seen_dot {
                frac_digits += 1;
                if frac_digits > 4 {
                    if digit != 0 {
                        return Err(RowError::BadAmount);
                    }
                    seen_digit = true;
                    continue;
                }
            }
            let Some(next) = units.checked_mul(10).and_then(|u| u.checked_add(i64::from(digit))) else {
                return Err(RowError::BadAmount);
            };
            units = next;
            seen_digit = true;
        }
        if !seen_digit {
            return Err(RowError::BadAmount);
        }
        for _ in frac_digits..4 {
            let Some(next) = units.checked_mul(10) else {
                return Err(RowError::BadAmount);
            };
            units = next;
        }
        Ok(Money(units))
    }

    pub fn checked_add(self, other: Money) -> Result<Money, RowError> {
        let Some(total) = self.0.checked_add(other.0) else {
            return Err(RowError::BadAmount);
        };
        Ok(Money(total))
    }

    pub fn checked_sub(self, other: Money) -> Result<Money, RowError> {
        let Some(total) = self.0.checked_sub(other.0) else {
            return Err(RowError::BadAmount);
        };
        Ok(Money(total))
    }

    pub fn sufficient(self, cost: Money) -> bool {
        self.0 >= cost.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn total_text(self, held: Money) -> String {
        let sum = i128::from(self.0) + i128::from(held.0);
        if sum > i128::from(i64::MAX) || sum < i128::from(i64::MIN) {
            return String::from("overflow");
        }
        let sign = if sum < 0 { "-" } else { "" };
        let magnitude = sum.unsigned_abs();
        let whole = magnitude.div_euclid(Self::SCALE_U128);
        let frac = magnitude.rem_euclid(Self::SCALE_U128);
        format!("{sign}{whole}.{frac:04}")
    }
}

impl Display for Money {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = self.0.unsigned_abs();
        let whole = magnitude.div_euclid(Self::SCALE_U64);
        let frac = magnitude.rem_euclid(Self::SCALE_U64);
        write!(f, "{sign}{whole}.{frac:04}")
    }
}
