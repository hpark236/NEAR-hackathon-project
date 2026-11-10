// Organic Score oracle (v1).
//
// Reads every holder of a INSYDR coin straight from the contract (public RPC, no indexer),
// groups wallets that look like one actor, counts independent holders who have held long enough,
// writes the evidence to app/data/score-<coin>.json, then signs and submits an attestation.
//
// Heuristics in v1 (stated plainly; see README "Limits"):
//   1. Shared parent: sub-accounts of the same parent (f1.farm.testnet, f2.farm.testnet, ...) and
//      the parent itself are one actor.
//   2. Deployer side: the creator, the launch receiver and anything under them are one cluster
//      ("deployer") and never count as independent.
// Not caught in v1: unrelated-looking top-level accounts funded through exchanges or bridges.
//
// usage: node oracle.js <coin-contract> <relayer-account> [--dry]
import { view, call, retry, getKey, KEY_DIR, readJson, writeJson } from "./lib/near.js";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const ORACLE_ID = "oracle.local";
const TOP_LEVEL = new Set(["testnet", "near"]);

const parentOf = (id) => {
  const i = id.indexOf(".");
  if (i < 0) return null; // implicit account
  const p = id.slice(i + 1);
  return TOP_LEVEL.has(p) ? null : p;
};

export async function allHolders(coin) {
  const out = [];
  for (let from = 0; ; from += 200) {
    const page = await view(coin, "get_holders", { from_index: from, limit: 200 });
    out.push(...page.map(([account_id, balance, since]) => ({ account_id, balance: BigInt(balance), since: Number(since) })));
    if (page.length < 200) return out;
  }
}

export async function score(coin) {
  const [cfg, holders, creator, launchReceiver] = await Promise.all([
    view(coin, "get_config"),
    allHolders(coin),
    view(coin, "get_creator"),
    view(coin, "get_launch_receiver"),
  ]);
  const prevState = readJson(path.join(here, "data", `score-${coin}.json`), { clusters: {} });
  const deployerRoots = new Set([creator, launchReceiver]);

  // union-find over holder ids
  const parent = new Map(holders.map((h) => [h.account_id, h.account_id]));
  const find = (x) => (parent.get(x) === x ? x : (parent.set(x, find(parent.get(x))), parent.get(x)));
  const union = (a, b) => parent.set(find(a), find(b));
  const reasons = new Map();
  const byParent = new Map();
  for (const h of holders) {
    const p = parentOf(h.account_id);
    if (p) {
      if (!byParent.has(p)) byParent.set(p, []);
      byParent.get(p).push(h.account_id);
    }
  }
  for (const [p, kids] of byParent) {
    const members = parent.has(p) ? [p, ...kids] : kids;
    if (members.length < 2) continue;
    for (const k of members.slice(1)) union(members[0], k);
    for (const k of members) reasons.set(k, `shares parent account ${p} with ${members.length - 1} other holder(s)`);
  }
  const isDeployer = (id) => [...deployerRoots].some((r) => id === r || id.endsWith("." + r));
  const deployerIds = holders.map((h) => h.account_id).filter(isDeployer);
  for (const k of deployerIds.slice(1)) union(deployerIds[0], k);
  for (const k of deployerIds) reasons.set(k, "creator / launch side");

  const groups = new Map();
  for (const h of holders) {
    const r = find(h.account_id);
    if (!groups.has(r)) groups.set(r, []);
    groups.get(r).push(h);
  }

  const now = Date.now();
  const holdMs = (cfg.age_full_days ?? 14) * Number(cfg.day_ms);
  let independent = 0;
  const clusters = [];
  let clusteredBalance = 0n;
  const total = holders.reduce((s, h) => s + h.balance, 0n);
  for (const [root, members] of groups) {
    const deployer = members.some((m) => isDeployer(m.account_id));
    const id = deployer ? "deployer" : `c:${root}`;
    const heldLongEnough = members.some((m) => m.since > 0 && now - m.since >= holdMs);
    if (!deployer && heldLongEnough) independent += 1;
    if (members.length > 1 || deployer) {
      clusteredBalance += members.reduce((s, m) => s + m.balance, 0n);
      // a deployer cluster always shares one weight, even if it is a single account
      const size = Math.max(members.length, deployer ? 1000 : 1);
      for (const m of members) {
        clusters.push({ account_id: m.account_id, cluster_id: id, size, reason: reasons.get(m.account_id) });
      }
    }
  }
  // clear flags for accounts that are no longer clustered
  const now_ids = new Set(clusters.map((c) => c.account_id));
  for (const id of Object.keys(prevState.clusters)) {
    if (!now_ids.has(id)) clusters.push({ account_id: id, cluster_id: "none", size: 1, reason: "no longer clustered" });
  }

  const report = {
    coin,
    ts: now,
    holders: holders.length,
    independent_holders: independent,
    hold_requirement_ms: holdMs,
    clustered_supply_share: total ? Number((clusteredBalance * 10000n) / total) / 100 : 0,
    largest_cluster: Math.max(0, ...[...groups.values()].filter((g) => g.length > 1).map((g) => g.length)),
    clusters: Object.fromEntries(clusters.filter((c) => c.size > 1).map((c) => [c.account_id, c])),
    heuristics: ["shared parent account", "creator / launch side"],
  };
  return { report, clusters };
}

export async function submit(coin, relayer, { dry = false } = {}) {
  const { report, clusters } = await score(coin);
  writeJson(path.join(here, "data", `score-${coin}.json`), report);
  const payload = JSON.stringify({
    ts: String(report.ts),
    independent_holders: report.independent_holders,
    clusters: clusters.map(({ account_id, cluster_id, size }) => ({ account_id, cluster_id, size })),
  });
  const kp = await getKey(ORACLE_ID);
  if (!kp) throw new Error(`no oracle key in ${KEY_DIR}; run setup first`);
  const { signature } = kp.sign(Buffer.from(payload));
  if (!dry) {
    // retrying is safe: if a lost response actually landed, the contract rejects the replay
    await retry(() => call(relayer, coin, "attest", { payload, signature: Buffer.from(signature).toString("base64") }, { gasTgas: 300 }))
      .catch((e) => { if (!/replay|stale|not newer/i.test(String(e.message || e))) throw e; });
  }
  return report;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [coin, relayer] = process.argv.slice(2);
  if (!coin || !relayer) {
    console.error("usage: node oracle.js <coin-contract> <relayer-account> [--dry]");
    process.exit(1);
  }
  const r = await submit(coin, relayer, { dry: process.argv.includes("--dry") });
  console.log(JSON.stringify({ ...r, clusters: Object.keys(r.clusters).length + " flagged accounts" }, null, 2));
}
