use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;

use crate::accounts::{Account, AccountBook, AccountRole};
use crate::amount::{Amount, Bps};
use crate::asset::{Asset, AssetBook};
use crate::catalog;
use crate::codec::pretty_json;
use crate::error::{TempestError, TempestResult};
use crate::ids::{AccountId, AssetId, Digest, MarketId};
use crate::liquidity::{HopRequest, LiquidityBook, Pool, PoolSummary};
use crate::operations::{OperationsMonitor, OperationsReport};
use crate::oracle::{MarketMove, MarketObservation, OracleBook};
use crate::pause::PauseController;
use crate::risk::{RiskConfig, RiskEngine};
use crate::routes::{RouteIntent, RoutePlanSummary, Router};
use crate::settlement::SettlementReceipt;

const NETWORK_ID: u64 = 91_707;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioReport {
    pub scenario: String,
    pub network_id: u64,
    pub slot: u64,
    pub mode: String,
    pub route_admitted: bool,
    pub rejection: Option<String>,
    pub ticket: Option<RoutePlanSummary>,
    pub receipt: Option<ReceiptSummary>,
    pub balances: BTreeMap<String, BTreeMap<String, i128>>,
    pub pools: Vec<PoolSummary>,
    pub moves: Vec<MoveSummary>,
    pub events: Vec<EventSummary>,
    pub event_count: usize,
    pub journal_count: usize,
    pub state_digest: Digest,
    pub catalog_size: usize,
    pub asset_symbols: Vec<String>,
    pub operations: OperationsReport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveSummary {
    pub market: String,
    pub price_move_bps: u32,
    pub depth_move_bps: u32,
    pub version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventSummary {
    pub slot: u64,
    pub kind: String,
    pub note: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReceiptSummary {
    pub ticket: String,
    pub source_amount: i128,
    pub destination_amount: i128,
    pub hops: usize,
    pub digest: String,
}

pub fn run() -> TempestResult<()> {
    let scenario = env::args().nth(1).unwrap_or_else(|| "normal".to_string());
    let report = run_scenario(&scenario)?;
    println!("{}", pretty_json(&report)?);
    Ok(())
}

pub fn run_scenario(name: &str) -> TempestResult<ScenarioReport> {
    match name {
        "normal" => scenario_normal(),
        "volatility" => scenario_volatility(),
        "pause" => scenario_pause(),
        "reactivation" => scenario_reactivation(),
        "ticket" => scenario_ticket(),
        "queued" => scenario_queued(),
        other => Err(TempestError::RouteRejected(format!(
            "unknown scenario {other}"
        ))),
    }
}

fn scenario_normal() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let intent = standard_intent(Amount::from_units(10_000), "normal-001")?;
    let ticket = router.plan_route(intent)?;
    let receipt = router.settle_ticket(&ticket.id)?;
    build_report(
        "normal",
        &router,
        true,
        None,
        Some(ticket.summary()),
        Some(receipt_summary(receipt)),
        Vec::new(),
    )
}

fn scenario_volatility() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let sol_usdc = MarketId::from("SOL-USDC");
    let sol_eurc = MarketId::from("SOL-EURC");
    let first = router.observe_market(
        &sol_usdc,
        52_000_000,
        Amount::from_units(1_210_000),
        88,
        Bps::new(190).unwrap(),
    )?;
    router.advance_slot(1);
    let second = router.observe_market(
        &sol_eurc,
        18_900_000,
        Amount::from_units(1_006_000),
        84,
        Bps::new(230).unwrap(),
    )?;
    build_report(
        "volatility",
        &router,
        true,
        None,
        None,
        None,
        vec![move_summary(first), move_summary(second)],
    )
}

fn scenario_pause() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let sol_usdc = MarketId::from("SOL-USDC");
    let market_move = router.observe_market(
        &sol_usdc,
        64_000_000,
        Amount::from_units(760_000),
        61,
        Bps::new(910).unwrap(),
    )?;
    let intent = standard_intent(Amount::from_units(8_000), "pause-001")?;
    let (route_admitted, rejection, ticket) = match router.plan_route(intent) {
        Ok(ticket) => (true, None, Some(ticket.summary())),
        Err(error) => {
            router.reject_note(Some(sol_usdc), error.to_string());
            (false, Some(error.to_string()), None)
        }
    };
    build_report(
        "pause",
        &router,
        route_admitted,
        rejection,
        ticket,
        None,
        vec![move_summary(market_move)],
    )
}

fn scenario_reactivation() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let sol_usdc = MarketId::from("SOL-USDC");
    let shock = router.observe_market(
        &sol_usdc,
        62_500_000,
        Amount::from_units(790_000),
        63,
        Bps::new(890).unwrap(),
    )?;
    router.advance_slot(1);
    let calm = router.observe_market(
        &sol_usdc,
        62_000_000,
        Amount::from_units(805_000),
        76,
        Bps::new(140).unwrap(),
    )?;
    router.advance_protection(3, true);
    router.advance_protection(3, true);

    let intent = standard_intent(Amount::from_units(7_500), "reactivation-001")?;
    let ticket = router.plan_route(intent)?;
    let receipt = router.settle_ticket(&ticket.id)?;
    build_report(
        "reactivation",
        &router,
        true,
        None,
        Some(ticket.summary()),
        Some(receipt_summary(receipt)),
        vec![move_summary(shock), move_summary(calm)],
    )
}

fn scenario_ticket() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let sol_eurc = MarketId::from("SOL-EURC");
    let intent = standard_intent(Amount::from_units(9_000), "ticket-001")?;
    let ticket = router.plan_route(intent)?;
    router.advance_slot(1);
    let market_move = router.observe_market(
        &sol_eurc,
        18_080_000,
        Amount::from_units(1_003_000),
        86,
        Bps::new(160).unwrap(),
    )?;
    let receipt = router.settle_ticket(&ticket.id)?;
    build_report(
        "ticket",
        &router,
        true,
        None,
        Some(ticket.summary()),
        Some(receipt_summary(receipt)),
        vec![move_summary(market_move)],
    )
}

fn scenario_queued() -> TempestResult<ScenarioReport> {
    let mut router = default_router()?;
    let first = router.plan_route(standard_intent(Amount::from_units(8_000), "queue-001")?)?;
    router.plan_route(standard_intent(Amount::from_units(9_000), "queue-002")?)?;
    build_report(
        "queued",
        &router,
        true,
        None,
        Some(first.summary()),
        None,
        Vec::new(),
    )
}

fn default_router() -> TempestResult<Router> {
    let usdc = AssetId::from("USDC");
    let sol = AssetId::from("SOL");
    let eurc = AssetId::from("EURC");
    let payer = AccountId::from("payer");
    let beneficiary = AccountId::from("beneficiary");
    let solver = AccountId::from("solver");
    let reserve = AccountId::from("reserve");

    let mut assets = AssetBook::new();
    assets.register(Asset::new(usdc.clone(), "USDC", 6, "stable"));
    assets.register(Asset::new(sol.clone(), "SOL", 9, "volatile"));
    assets.register(Asset::new(eurc.clone(), "EURC", 6, "stable"));

    let mut accounts = AccountBook::new();
    accounts.insert(Account::new(
        payer.clone(),
        AccountRole::Trader,
        Amount::from_units(200_000),
    ));
    accounts.insert(Account::new(
        beneficiary.clone(),
        AccountRole::Trader,
        Amount::from_units(200_000),
    ));
    accounts.insert(Account::new(
        solver,
        AccountRole::Solver,
        Amount::from_units(80_000),
    ));
    accounts.insert(Account::new(
        reserve,
        AccountRole::Reserve,
        Amount::from_units(1_000_000),
    ));
    accounts.credit(&payer, &usdc, Amount::from_units(100_000))?;
    accounts.credit(&beneficiary, &eurc, Amount::from_units(0))?;
    accounts.credit(&beneficiary, &sol, Amount::from_units(0))?;

    let mut liquidity = LiquidityBook::new();
    liquidity.insert(Pool::new(
        "SOL-USDC",
        usdc.clone(),
        sol.clone(),
        Amount::from_units(1_200_000),
        Amount::from_units(60_000),
        Bps::new(30).unwrap(),
    ));
    liquidity.insert(Pool::new(
        "SOL-EURC",
        sol.clone(),
        eurc.clone(),
        Amount::from_units(58_000),
        Amount::from_units(1_020_000),
        Bps::new(25).unwrap(),
    ));

    let mut oracle = OracleBook::new();
    oracle.insert(MarketObservation::new(
        "SOL-USDC",
        1,
        50_000_000,
        Amount::from_units(1_260_000),
        92,
        Bps::new(120).unwrap(),
    ));
    oracle.insert(MarketObservation::new(
        "SOL-EURC",
        1,
        18_000_000,
        Amount::from_units(1_078_000),
        90,
        Bps::new(130).unwrap(),
    ));

    let pause = PauseController::new(
        Bps::new(900).unwrap(),
        Bps::new(1_600).unwrap(),
        Bps::new(700).unwrap(),
        3,
    );
    let risk = RiskEngine::new(RiskConfig::production());
    Ok(Router::new(
        assets, accounts, liquidity, oracle, pause, risk,
    ))
}

fn standard_intent(amount_in: Amount, client_order_id: &str) -> TempestResult<RouteIntent> {
    Ok(RouteIntent {
        payer: AccountId::from("payer"),
        beneficiary: AccountId::from("beneficiary"),
        asset_in: AssetId::from("USDC"),
        asset_out: AssetId::from("EURC"),
        amount_in,
        max_deviation_bps: Bps::new(200).unwrap(),
        path: vec![
            HopRequest::new("SOL-USDC", "USDC", "SOL"),
            HopRequest::new("SOL-EURC", "SOL", "EURC"),
        ],
        client_order_id: client_order_id.to_string(),
    })
}

fn build_report(
    scenario: &str,
    router: &Router,
    route_admitted: bool,
    rejection: Option<String>,
    ticket: Option<RoutePlanSummary>,
    receipt: Option<ReceiptSummary>,
    moves: Vec<MoveSummary>,
) -> TempestResult<ScenarioReport> {
    let mut balances = BTreeMap::new();
    for asset in ["USDC", "SOL", "EURC"] {
        let asset_id = AssetId::from(asset);
        balances.insert(asset.to_string(), router.balances_for_asset(&asset_id));
    }
    let events = router
        .events()
        .all()
        .iter()
        .map(|event| EventSummary {
            slot: event.slot,
            kind: format!("{:?}", event.kind),
            note: event.note.clone(),
        })
        .collect::<Vec<_>>();
    Ok(ScenarioReport {
        scenario: scenario.to_string(),
        network_id: NETWORK_ID,
        slot: router.slot(),
        mode: router.mode_label().to_string(),
        route_admitted,
        rejection,
        ticket,
        receipt,
        balances,
        pools: router.pool_summaries(),
        moves,
        events,
        event_count: router.events().len(),
        journal_count: router.journal().len(),
        state_digest: router.state_digest(),
        catalog_size: catalog::RISK_PRESETS.len(),
        asset_symbols: router.assets().symbols(),
        operations: OperationsMonitor.snapshot(router)?,
    })
}

fn move_summary(market_move: MarketMove) -> MoveSummary {
    MoveSummary {
        market: market_move.market.to_string(),
        price_move_bps: market_move.price_move_bps.raw(),
        depth_move_bps: market_move.depth_move_bps.raw(),
        version: market_move.version,
    }
}

fn receipt_summary(receipt: SettlementReceipt) -> ReceiptSummary {
    ReceiptSummary {
        ticket: receipt.ticket.to_string(),
        source_amount: receipt.source_amount.raw(),
        destination_amount: receipt.destination_amount.raw(),
        hops: receipt.hops,
        digest: receipt.digest.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_scenario_settles() {
        let report = run_scenario("normal").unwrap();
        assert_eq!(report.mode, "open");
        assert!(report.route_admitted);
        assert_eq!(report.receipt.as_ref().unwrap().hops, 2);
        assert!(report.receipt.as_ref().unwrap().destination_amount > 0);
    }

    #[test]
    fn pause_scenario_rejects_admission() {
        let report = run_scenario("pause").unwrap();
        assert_eq!(report.mode, "paused");
        assert!(!report.route_admitted);
        assert!(report.rejection.unwrap().contains("paused"));
    }

    #[test]
    fn queued_scenario_reports_market_pipeline() {
        let report = run_scenario("queued").unwrap();
        assert_eq!(report.operations.prepared_tickets, 2);
        assert_eq!(report.operations.settled_tickets, 0);
        assert_eq!(report.operations.pool_count, 2);
        assert_eq!(report.operations.market_pipeline.len(), 2);
        assert!(
            report
                .operations
                .market_pipeline
                .iter()
                .all(|line| line.prepared_routes == 2)
        );
    }
}
