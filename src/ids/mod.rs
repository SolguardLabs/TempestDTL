use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AccountId(String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AssetId(String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct MarketId(String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TicketId(String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Digest(String);

impl AccountId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AssetId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl MarketId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TicketId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn deterministic(parts: &[&str]) -> Self {
        Self(stable_hex("ticket", parts))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Digest {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn stable(namespace: &str, parts: &[&str]) -> Self {
        Self(stable_hex(namespace, parts))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for MarketId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for TicketId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for AccountId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AssetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&str> for MarketId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

fn stable_hex(namespace: &str, parts: &[&str]) -> String {
    let mut words = [
        0xcbf2_9ce4_8422_2325_u64,
        0x9e37_79b9_7f4a_7c15_u64,
        0x94d0_49bb_1331_11eb_u64,
        0xd6e8_feb8_6659_fd93_u64,
    ];
    for (index, word) in words.iter_mut().enumerate() {
        mix_bytes(word, namespace.as_bytes(), index as u64 + 1);
        for part in parts {
            mix_bytes(word, part.as_bytes(), index as u64 + 17);
            mix_bytes(word, &[0xff], index as u64 + 29);
        }
    }
    format!(
        "{:016x}{:016x}{:016x}{:016x}",
        words[0], words[1], words[2], words[3]
    )
}

fn mix_bytes(state: &mut u64, bytes: &[u8], salt: u64) {
    *state ^= salt.wrapping_mul(0x100_0000_01b3);
    for byte in bytes {
        *state ^= u64::from(*byte);
        *state = state.wrapping_mul(0x100_0000_01b3);
        *state ^= *state >> 32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_has_64_hex_chars() {
        let digest = Digest::stable("state", &["alpha", "beta"]);
        assert_eq!(digest.as_str().len(), 64);
        assert!(digest.as_str().chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
