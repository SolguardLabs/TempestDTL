import { writeFileSync } from "node:fs";
import { join } from "node:path";

const markets = [
  "SOL-USDC",
  "SOL-EURC",
  "BTC-USDC",
  "ETH-USDC",
  "JITO-SOL",
  "BONK-USDC",
  "PYTH-USDC",
  "JUP-USDC",
  "USDT-USDC",
  "EURC-USDC",
  "MSOL-SOL",
  "BSOL-SOL",
  "WIF-USDC",
  "HNT-USDC",
  "RAY-USDC",
  "ORCA-USDC",
  "DRIFT-USDC",
  "WETH-SOL",
];

const venues = ["tempest-core", "tempest-rfq", "tempest-amm"];
const tiers = ["core", "edge", "auction", "overflow", "private"];

const entries = [];
let index = 0;
for (const venue of venues) {
  for (const market of markets) {
    for (const tier of tiers) {
      const tierIndex = tiers.indexOf(tier);
      const venueIndex = venues.indexOf(venue);
      const marketIndex = markets.indexOf(market);
      const preset = {
        venue,
        market,
        tier,
        maxDeviation: 190 + ((marketIndex * 7 + tierIndex * 13 + venueIndex * 11) % 95),
        maxVolatility: 560 + ((marketIndex * 19 + tierIndex * 23 + venueIndex * 17) % 210),
        minScore: 64 + ((marketIndex + tierIndex + venueIndex) % 22),
        minDepth: 28_000 + marketIndex * 1_350 + tierIndex * 2_200 + venueIndex * 3_000,
        ttl: 8 + ((marketIndex + tierIndex + venueIndex) % 9),
        floor: 5_000 + marketIndex * 330 + tierIndex * 410 + venueIndex * 720,
        weight: 50 + ((marketIndex * 5 + tierIndex * 7 + venueIndex * 9) % 50),
      };
      if (venue === "tempest-core" && market === "SOL-USDC" && tier === "core") {
        Object.assign(preset, {
          maxDeviation: 220,
          maxVolatility: 620,
          minScore: 72,
          minDepth: 40_000,
          ttl: 12,
          floor: 8_000,
          weight: 96,
        });
      }
      if (venue === "tempest-core" && market === "SOL-EURC" && tier === "core") {
        Object.assign(preset, {
          maxDeviation: 240,
          maxVolatility: 640,
          minScore: 70,
          minDepth: 35_000,
          ttl: 12,
          floor: 7_000,
          weight: 92,
        });
      }
      entries.push({ ...preset, index: index++ });
    }
  }
}

const lines = [];
lines.push("use crate::risk::RiskPreset;");
lines.push("");
lines.push(`pub static RISK_PRESETS: [RiskPreset; ${entries.length}] = [`);
for (const entry of entries) {
  lines.push("    RiskPreset {");
  lines.push(`        venue: "${entry.venue}",`);
  lines.push(`        market_code: "${entry.market}",`);
  lines.push(`        tier: "${entry.tier}",`);
  lines.push(`        max_deviation_bps: ${entry.maxDeviation},`);
  lines.push(`        max_volatility_bps: ${entry.maxVolatility},`);
  lines.push(`        min_liquidity_score: ${entry.minScore},`);
  lines.push(`        min_depth_units: ${entry.minDepth},`);
  lines.push(`        quote_ttl_slots: ${entry.ttl},`);
  lines.push(`        reserve_floor_units: ${entry.floor},`);
  lines.push(`        route_weight: ${entry.weight},`);
  lines.push("    },");
}
lines.push("];");
lines.push("");

writeFileSync(join("src", "catalog", "generated.rs"), lines.join("\n"));
