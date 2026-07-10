import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const cache = new Map();

export function scenario(name) {
  if (!cache.has(name)) {
    const stdout = execFileSync(cargo, ["run", "--quiet", "--", name], {
      cwd: root,
      encoding: "utf8",
      env: { ...process.env, CARGO_TERM_COLOR: "never" },
      timeout: 120_000,
    });
    cache.set(name, JSON.parse(stdout));
  }
  return cache.get(name);
}

export function assertHex64(value) {
  assert.equal(typeof value, "string");
  assert.match(value, /^[0-9a-f]{64}$/u);
}

export function assertCommon(payload, expectedScenario) {
  assert.equal(payload.scenario, expectedScenario);
  assert.equal(payload.network_id, 91_707);
  assert.equal(typeof payload.slot, "number");
  assert.equal(typeof payload.mode, "string");
  assert.equal(typeof payload.event_count, "number");
  assert.equal(typeof payload.journal_count, "number");
  assert.ok(Array.isArray(payload.events));
  assert.ok(Array.isArray(payload.pools));
  assert.ok(payload.catalog_size >= 2);
  assertHex64(payload.state_digest);
}
