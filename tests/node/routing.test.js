import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, scenario } from "../helpers/scenario_helpers.js";

test("normal liquida una ruta multi-hop y actualiza balances", () => {
  const payload = scenario("normal");
  assertCommon(payload, "normal");

  assert.equal(payload.mode, "open");
  assert.equal(payload.route_admitted, true);
  assert.equal(payload.ticket.hops, 2);
  assert.equal(payload.receipt.hops, 2);
  assert.equal(payload.receipt.source_amount, 10_000);
  assert.ok(payload.receipt.destination_amount > 0);
  assert.equal(payload.balances.USDC.payer, 90_000);
  assert.equal(payload.balances.EURC.beneficiary, payload.receipt.destination_amount);
});

test("ticket mantiene lifecycle determinista con una actualizacion moderada", () => {
  const payload = scenario("ticket");
  assertCommon(payload, "ticket");

  assert.equal(payload.route_admitted, true);
  assert.equal(payload.mode, "open");
  assert.equal(payload.moves.length, 1);
  assert.ok(payload.moves[0].price_move_bps < 900);
  assert.equal(payload.receipt.source_amount, 9_000);
  assert.equal(payload.journal_count, 2);
});
