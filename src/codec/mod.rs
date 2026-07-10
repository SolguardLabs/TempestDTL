use serde::Serialize;

use crate::error::TempestResult;
use crate::ids::Digest;

pub fn to_canonical_json<T: Serialize>(value: &T) -> TempestResult<String> {
    Ok(serde_json::to_string(value)?)
}

pub fn pretty_json<T: Serialize>(value: &T) -> TempestResult<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

pub fn digest_json<T: Serialize>(namespace: &str, value: &T) -> TempestResult<Digest> {
    let json = to_canonical_json(value)?;
    Ok(Digest::stable(namespace, &[&json]))
}
