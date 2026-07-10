use crate::risk::RiskPreset;

mod generated;

pub use generated::RISK_PRESETS;

pub fn find_preset(market_code: &str, tier: &str) -> Option<RiskPreset> {
    RISK_PRESETS
        .iter()
        .copied()
        .find(|preset| preset.market_code == market_code && preset.tier == tier)
}

pub fn presets_for_market(market_code: &str) -> Vec<RiskPreset> {
    RISK_PRESETS
        .iter()
        .copied()
        .filter(|preset| preset.market_code == market_code)
        .collect()
}

pub fn catalog_digest_parts() -> Vec<String> {
    RISK_PRESETS
        .iter()
        .map(|preset| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                preset.venue,
                preset.market_code,
                preset.tier,
                preset.max_deviation_bps,
                preset.max_volatility_bps,
                preset.min_liquidity_score,
                preset.min_depth_units,
                preset.quote_ttl_slots,
                preset.reserve_floor_units,
                preset.route_weight
            )
        })
        .collect()
}
