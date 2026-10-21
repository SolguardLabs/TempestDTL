import assert from "node:assert/strict";
import test from "node:test";

import { assertCommon, scenario } from "../helpers/scenario_helpers.js";

test("queued resume compromisos pendientes por mercado y dirección", () => {
  const payload = scenario("queued");
  assertCommon(payload, "queued");
  assert.equal(payload.operations.prepared_tickets, 2);
  assert.equal(payload.operations.settled_tickets, 0);
  assert.equal(payload.operations.pool_count, 2);
  assert.equal(payload.operations.market_pipeline.length, 2);
  assert.ok(payload.operations.market_pipeline.every((line) => line.prepared_routes === 2));
  assert.ok(payload.operations.market_pipeline.every((line) => line.input_units > 0));
});

test("normal elimina el compromiso del pipeline tras liquidación", () => {
  const payload = scenario("normal");
  assertCommon(payload, "normal");
  assert.equal(payload.operations.prepared_tickets, 0);
  assert.equal(payload.operations.settled_tickets, 1);
  assert.deepEqual(payload.operations.market_pipeline, []);
});
