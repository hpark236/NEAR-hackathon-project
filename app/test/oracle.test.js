// Unit tests for the Organic Score clustering (pure function, no chain needed):  npm test
import test from "node:test";
import assert from "node:assert/strict";
import { scoreHolders, attestationPayload, parentOf } from "../oracle.js";

const DAY = 86_400_000;
const NOW = 100 * DAY;
const T = 10n ** 18n;
const cfg = { holder_min_balance: (1_000n * T).toString(), holder_min_days: 14, day_ms: String(DAY) };
const h = (account_id, tokens, source = "launch.creator.near", ageDays = 30) =>
  ({ account_id, balance: BigInt(tokens) * T, since_ms: NOW - ageDays * DAY, source });
const run = (holders, extra = {}) =>
  scoreHolders({ cfg, holders, creator: "creator.near", launchReceiver: "launch.creator.near", now: NOW, ...extra });
const cluster = (r, id) => r.report.clusters[id]?.cluster_id;

test("independent buyers from the market are not clustered", () => {
  const r = run([h("alice.near", 5000), h("bob.near", 5000), h("a".repeat(64), 5000)]);
  assert.equal(r.report.independent_holders, 3);
  assert.deepEqual(r.report.clusters, {});
});

test("sub-accounts of one parent are one holder", () => {
  const r = run([h("f1.farm.near", 5000), h("f2.farm.near", 5000), h("f3.farm.near", 5000), h("alice.near", 5000)]);
  assert.equal(r.report.independent_holders, 2);
  assert.equal(r.report.clusters["f1.farm.near"].size, 3);
  assert.equal(cluster(r, "f1.farm.near"), cluster(r, "f3.farm.near"));
});

test("wallets fed from one stack are one holder, even with unrelated names", () => {
  const farm = ["a", "b", "c", "d"].map((c) => h(c.repeat(64), 5000, "farmer.near"));
  const r = run([h("farmer.near", 1000), ...farm, h("alice.near", 5000)]);
  assert.equal(r.report.independent_holders, 2, "the farm counts once, alice once");
  assert.equal(r.report.clusters["a".repeat(64)].size, 5);
  assert.match(r.report.clusters["a".repeat(64)].reason, /most of its coins came from farmer\.near/);
});

test("a farm is still one holder after the farmer empties their own wallet", () => {
  const farm = ["a", "b", "c"].map((c) => h(c.repeat(64), 5000, "farmer.near"));
  const r = run([...farm, h("alice.near", 5000)]);
  assert.equal(r.report.independent_holders, 2);
  assert.equal(r.report.clusters["b".repeat(64)].size, 3);
});

test("source chains are followed through a relay wallet", () => {
  const relay = h("relay.near", 10, "farmer.near");
  const farm = ["a", "b"].map((c) => h(c.repeat(64), 5000, "relay.near"));
  const r = run([h("farmer.near", 5000), relay, ...farm]);
  assert.equal(r.report.independent_holders, 1);
  assert.equal(cluster(r, "a".repeat(64)), cluster(r, "farmer.near"));
});

test("one friend tipping one wallet is not a farm", () => {
  // bob's coins mostly came from alice, who no longer holds any: a gift, not a split stack
  const r = run([h("bob.near", 5000, "alice.near"), h("carol.near", 5000)]);
  assert.equal(r.report.independent_holders, 2);
  assert.deepEqual(r.report.clusters, {});
});

test("creator side never counts: sub-accounts, the launch account and wallets the creator fed", () => {
  const r = run([
    h("launch.creator.near", 1_000_000, null),
    h("creator.near", 5000),
    h("insider1.creator.near", 400_000),
    h("9".repeat(64), 400_000, "creator.near"), // a "stealth" insider wallet
    h("alice.near", 5000),
  ]);
  assert.equal(r.report.independent_holders, 1);
  for (const id of ["launch.creator.near", "creator.near", "insider1.creator.near", "9".repeat(64)]) {
    assert.equal(cluster(r, id), "deployer", id);
    assert.equal(r.report.clusters[id].size, 1000);
  }
});

test("dust and fresh coins do not count as holders", () => {
  const r = run([h("dust.near", 10), h("fresh.near", 5000, "launch.creator.near", 3), h("alice.near", 5000)]);
  assert.equal(r.report.independent_holders, 1);
});

test("a cluster counts once if any member qualifies", () => {
  const r = run([h("f1.farm.near", 10), h("f2.farm.near", 5000)]);
  assert.equal(r.report.independent_holders, 1);
});

test("flags are cleared when an account stops being clustered", () => {
  const r = run([h("alice.near", 5000)], { previouslyFlagged: ["alice.near", "gone.near"] });
  assert.deepEqual(r.clusters.map((c) => [c.account_id, c.size]), [["alice.near", 1], ["gone.near", 1]]);
});

test("scoring is deterministic: holder order does not change the signed payload", () => {
  const hs = [h("f1.farm.near", 5000), h("f2.farm.near", 5000), h("alice.near", 5000), h("z".repeat(64), 5000, "f1.farm.near")];
  const a = run(hs), b = run([...hs].reverse());
  const strip = (r) => attestationPayload(r.report, r.clusters).replace(/"cluster_id":"c:[^"]+"/g, '"cluster_id":"c"');
  assert.equal(strip(a), strip(b));
  assert.equal(a.report.independent_holders, b.report.independent_holders);
});

test("market accounts link nobody", () => {
  const r = run([h("a.near", 5000, "pool.dex.near"), h("b.near", 5000, "pool.dex.near")], { market: ["pool.dex.near"] });
  assert.equal(r.report.independent_holders, 2);
});

test("parentOf treats network roots as top level", () => {
  assert.equal(parentOf("alice.testnet"), null);
  assert.equal(parentOf("f1.farm.testnet"), "farm.testnet");
  assert.equal(parentOf("a".repeat(64)), null);
});
