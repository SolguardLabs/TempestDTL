import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, scenario } from "../helpers/scenario_helpers.js";

test("cambios moderados de volatilidad no cierran admision", () => {
  const payload = scenario("volatility");
  assertCommon(payload, "volatility");

  assert.equal(payload.mode, "open");
  assert.equal(payload.moves.length, 2);
  assert.ok(payload.moves.every((move) => move.price_move_bps < 900));
  assert.ok(payload.moves.every((move) => move.depth_move_bps < 1600));
});

test("shock de mercado activa pausa y rechaza nuevas rutas", () => {
  const payload = scenario("pause");
  assertCommon(payload, "pause");

  assert.equal(payload.mode, "paused");
  assert.equal(payload.route_admitted, false);
  assert.match(payload.rejection, /paused/u);
  assert.equal(payload.ticket, null);
  assert.equal(payload.receipt, null);
});

test("ventana estable permite reactivacion y settlement posterior", () => {
  const payload = scenario("reactivation");
  assertCommon(payload, "reactivation");

  assert.equal(payload.mode, "open");
  assert.equal(payload.route_admitted, true);
  assert.equal(payload.moves.length, 2);
  assert.ok(payload.events.some((event) => event.kind === "PauseEntered"));
  assert.ok(payload.events.some((event) => event.kind === "PauseRecovered"));
  assert.ok(payload.receipt.destination_amount > 0);
});
