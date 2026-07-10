use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::error::{TempestError, TempestResult};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash,
)]
pub struct Amount(i128);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Bps(u32);

impl Amount {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub fn new(value: i128) -> TempestResult<Self> {
        if value < 0 {
            return Err(TempestError::InvalidAmount);
        }
        Ok(Self(value))
    }

    pub fn from_units(value: i128) -> Self {
        debug_assert!(value >= 0);
        Self(value)
    }

    pub fn raw(self) -> i128 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: Self) -> TempestResult<Self> {
        self.0
            .checked_add(rhs.0)
            .filter(|value| *value >= 0)
            .map(Self)
            .ok_or(TempestError::InvalidAmount)
    }

    pub fn checked_sub(self, rhs: Self) -> TempestResult<Self> {
        self.0
            .checked_sub(rhs.0)
            .filter(|value| *value >= 0)
            .map(Self)
            .ok_or(TempestError::InvalidAmount)
    }

    pub fn checked_mul_u32(self, rhs: u32) -> TempestResult<Self> {
        self.0
            .checked_mul(i128::from(rhs))
            .filter(|value| *value >= 0)
            .map(Self)
            .ok_or(TempestError::InvalidAmount)
    }

    pub fn checked_mul_i128(self, rhs: i128) -> TempestResult<Self> {
        self.0
            .checked_mul(rhs)
            .filter(|value| *value >= 0)
            .map(Self)
            .ok_or(TempestError::InvalidAmount)
    }

    pub fn checked_div_i128(self, rhs: i128) -> TempestResult<Self> {
        if rhs <= 0 {
            return Err(TempestError::InvalidAmount);
        }
        self.0
            .checked_div(rhs)
            .filter(|value| *value >= 0)
            .map(Self)
            .ok_or(TempestError::InvalidAmount)
    }

    pub fn apply_bps_down(self, bps: Bps) -> TempestResult<Self> {
        let numerator = self
            .0
            .checked_mul(i128::from(bps.raw()))
            .ok_or(TempestError::InvalidAmount)?;
        Ok(Self(numerator / 10_000))
    }

    pub fn minus_bps(self, bps: Bps) -> TempestResult<Self> {
        self.checked_sub(self.apply_bps_down(bps)?)
    }

    pub fn plus_bps(self, bps: Bps) -> TempestResult<Self> {
        self.checked_add(self.apply_bps_down(bps)?)
    }

    pub fn min(self, rhs: Self) -> Self {
        match self.cmp(&rhs) {
            Ordering::Greater => rhs,
            _ => self,
        }
    }

    pub fn max(self, rhs: Self) -> Self {
        match self.cmp(&rhs) {
            Ordering::Less => rhs,
            _ => self,
        }
    }
}

impl Bps {
    pub const ZERO: Self = Self(0);
    pub const ONE_PERCENT: Self = Self(100);
    pub const FULL: Self = Self(10_000);

    pub fn new(value: u32) -> TempestResult<Self> {
        if value > 10_000 {
            return Err(TempestError::InvalidAmount);
        }
        Ok(Self(value))
    }

    pub fn raw(self) -> u32 {
        self.0
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0).min(10_000))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Bps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}bps", self.0)
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_bps_down() {
        let amount = Amount::from_units(10_000);
        assert_eq!(
            amount.apply_bps_down(Bps::new(75).unwrap()).unwrap().raw(),
            75
        );
        assert_eq!(
            amount.minus_bps(Bps::new(75).unwrap()).unwrap().raw(),
            9_925
        );
    }

    #[test]
    fn rejects_negative_amounts() {
        assert!(Amount::new(-1).is_err());
    }
}
