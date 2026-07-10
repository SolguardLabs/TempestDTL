use thiserror::Error;

pub type TempestResult<T> = Result<T, TempestError>;

#[derive(Debug, Error)]
pub enum TempestError {
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("asset not found: {0}")]
    AssetNotFound(String),
    #[error("market not found: {0}")]
    MarketNotFound(String),
    #[error("ticket not found: {0}")]
    TicketNotFound(String),
    #[error("invalid amount")]
    InvalidAmount,
    #[error("insufficient balance in {account} for {asset}")]
    InsufficientBalance { account: String, asset: String },
    #[error("insufficient reserve in market {market}")]
    InsufficientReserve { market: String },
    #[error("route rejected: {0}")]
    RouteRejected(String),
    #[error("router paused: {0}")]
    RouterPaused(String),
    #[error("ticket expired")]
    TicketExpired,
    #[error("ticket already settled")]
    TicketAlreadySettled,
    #[error("oracle observation missing: {0}")]
    OracleMissing(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for TempestError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
