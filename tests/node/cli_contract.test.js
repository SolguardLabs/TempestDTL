import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, assertHex64, scenario } from "../helpers/scenario_helpers.js";

test("los escenarios publicos serializan el contrato JSON esperado", () => {
  for (const name of ["normal", "volatility", "pause", "reactivation", "ticket"]) {
    const payload = scenario(name);
    assertCommon(payload, name);

    for (const key of [
      "route_admitted",
      "rejection",
      "ticket",
      "receipt",
      "balances",
      "moves",
      "asset_symbols",
    ]) {
      assert.ok(Object.hasOwn(payload, key), `${name} no incluye ${key}`);
    }
  }
});

test("tickets y receipts exponen digests estables cuando existen", () => {
  for (const name of ["normal", "reactivation", "ticket"]) {
    const payload = scenario(name);
    assert.equal(payload.route_admitted, true);
    assertHex64(payload.ticket.quote_digest);
    assertHex64(payload.ticket.admission_digest);
    assertHex64(payload.receipt.digest);
    assert.equal(payload.ticket.ticket, payload.receipt.ticket);
  }
});
