// Organic Score oracle (v2).
//
// Reads every holder of an INSYDR coin straight from the contract (public RPC, no indexer),
// groups wallets that look like one actor, counts independent holders, writes the evidence to
// app/data/score-<coin>.json, then signs and submits an attestation.
//
// The scoring is a pure function of on-chain state (`scoreHolders`), so every oracle signer that
// reads the same block computes the same payload, and anyone can re-run it to check a flag.
//
// Clustering rules (stated plainly; see docs/STRENGTH-TEST.md for what they miss):
//   1. Shared parent: sub-accounts of the same parent (f1.farm.testnet, f2.farm.testnet, ...) and
//      the parent itself are one actor.
//   2. Majority token source: the contract records, for every holder, the account that supplied
//      most of its coins. Wallets whose coins mostly came from the same non-market account are
//      one actor (one stack, split). Chains are followed to their root.
//   3. Creator side: the creator, the launch receiver, their sub-accounts and anything whose coins
//      mostly came from the creator are one cluster ("deployer") and never count as independent.
// An account counts as an independent holder only if it holds `holder_min_balance` with a
// coin-age of `holder_min_days` (both fixed in the coin's config).
//
// usage: node oracle.js <coin-contract> <relayer-account> [--dry]
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const ORACLE_IDS = ["oracle0.local", "oracle1.local", "oracle2.local"];
const TOP_LEVEL = new Set(["testnet", "near", "sandbox"]);

export const parentOf = (id) => {
  const i = id.indexOf(".");
  if (i < 0) return null; // implicit account
  const p = id.slice(i + 1);
  return TOP_LEVEL.has(p) ? null : p;
};

/**
 * Pure scoring. `holders`: [{account_id, balance: bigint, since_ms: number, source: string|null}].
 * `market`: accounts that sell to everyone (launch receiver, DEX pools); being supplied by one of
 * them links nobody.
 */
export function scoreHolders({ cfg, holders, creator, launchReceiver, market = [], now, previouslyFlagged = [] }) {
  const marketSet = new Set([launchReceiver, ...market]);
  const byId = new Map(holders.map((h) => [h.account_id, h]));
  const reasons = new Map();
  const note = (id, why) => { if (!reasons.has(id)) reasons.set(id, why); };

  // union-find over account ids (holders, plus sources that no longer hold anything)
  const parent = new Map();
  const add = (x) => { if (!parent.has(x)) parent.set(x, x); };
  const find = (x) => { add(x); let r = x; while (parent.get(r) !== r) r = parent.get(r); parent.set(x, r); return r; };
  const union = (a, b) => { const ra = find(a), rb = find(b); if (ra !== rb) parent.set(ra, rb); };
  holders.forEach((h) => add(h.account_id));

  // 1. shared parent
  const kidsOf = new Map();
  for (const h of holders) {
    const p = parentOf(h.account_id);
    if (!p) continue;
    if (!kidsOf.has(p)) kidsOf.set(p, []);
    kidsOf.get(p).push(h.account_id);
  }
  for (const [p, kids] of kidsOf) {
    const members = byId.has(p) ? [p, ...kids] : kids;
    if (members.length < 2) continue;
    for (const k of members.slice(1)) union(members[0], k);
    for (const k of members) note(k, `shares parent account ${p} with ${members.length - 1} other holder(s)`);
  }

  // 2. majority token source, followed to its root
  const rootSource = (id) => {
    const seen = new Set([id]);
    let cur = byId.get(id)?.source ?? null;
    let root = null;
    while (cur && !marketSet.has(cur) && !seen.has(cur)) {
      root = cur;
      seen.add(cur);
      cur = byId.get(cur)?.source ?? null;
    }
    return root;
  };
  const bySource = new Map();
  for (const h of holders) {
    if (marketSet.has(h.account_id)) continue;
    const r = rootSource(h.account_id);
    if (!r || r === h.account_id) continue;
    if (!bySource.has(r)) bySource.set(r, []);
    bySource.get(r).push(h.account_id);
  }
  for (const [src, kids] of bySource) {
    // one wallet fed by a friend is not a farm: link only when the source is itself a holder,
    // or feeds two or more wallets
    if (kids.length < 2 && !byId.has(src)) continue;
    for (const k of kids) {
      union(src, k);
      note(k, `most of its coins came from ${src}` + (kids.length > 1 ? `, like ${kids.length - 1} other holder(s)` : ""));
    }
    if (byId.has(src)) note(src, `supplied most of the coins of ${kids.length} other holder(s)`);
  }

  // 3. creator side
  const deployerRoots = [creator, launchReceiver];
  const underDeployer = (id) => deployerRoots.some((r) => id === r || id.endsWith("." + r));
  const deployerGroups = new Set();
  for (const id of parent.keys()) if (underDeployer(id)) deployerGroups.add(find(id));
  // a wallet whose coins mostly came from the creator is creator side too
  for (const h of holders) if (rootSource(h.account_id) === creator) deployerGroups.add(find(h.account_id));
  const isDeployerGroup = (root) => deployerGroups.has(root);

  const groups = new Map();
  for (const h of holders) {
    const r = find(h.account_id);
    if (!groups.has(r)) groups.set(r, []);
    groups.get(r).push(h);
  }

  const minBal = BigInt(cfg.holder_min_balance ?? "0");
  const holdMs = Number(cfg.holder_min_days ?? cfg.age_full_days ?? 14) * Number(cfg.day_ms);
  let independent = 0;
  const clusters = [];
  let clusteredBalance = 0n;
  let total = 0n;
  // normalise deployer groups first so several of them merge into one "deployer" cluster
  for (const [root, members] of groups) {
    const deployer = isDeployerGroup(root) || members.some((m) => underDeployer(m.account_id));
    const bal = members.reduce((s, m) => s + m.balance, 0n);
    if (!members.every((m) => marketSet.has(m.account_id))) total += bal;
    const counts = members.some((m) => m.balance >= minBal && m.since_ms > 0 && now - m.since_ms >= holdMs);
    if (!deployer && counts) independent += 1;
    if (members.length > 1 || deployer) {
      if (!members.every((m) => marketSet.has(m.account_id))) clusteredBalance += bal;
      // the creator side always shares one tiny weight, even as a single account
      const size = deployer ? 1000 : members.length;
      const id = deployer ? "deployer" : `c:${root}`;
      for (const m of members) {
        clusters.push({
          account_id: m.account_id,
          cluster_id: id,
          size,
          reason: reasons.get(m.account_id) ?? (deployer ? "creator / launch side" : "linked to a cluster"),
        });
      }
    }
  }
  clusters.sort((a, b) => (a.account_id < b.account_id ? -1 : 1));
  // clear flags for accounts that are no longer clustered
  const flaggedNow = new Set(clusters.map((c) => c.account_id));
  const cleared = [...previouslyFlagged].filter((id) => !flaggedNow.has(id)).sort()
    .map((id) => ({ account_id: id, cluster_id: "none", size: 1, reason: "no longer clustered" }));

  const multi = [...groups.entries()].filter(([r, g]) => g.length > 1 && !isDeployerGroup(r));
  const report = {
    ts: now,
    holders: holders.length,
    independent_holders: independent,
    holder_min_balance: minBal.toString(),
    hold_requirement_ms: holdMs,
    clustered_supply_share: total ? Number((clusteredBalance * 10000n) / total) / 100 : 0,
    farms: multi.length,
    largest_cluster: Math.max(0, ...multi.map(([, g]) => g.length)),
    clusters: Object.fromEntries(clusters.map((c) => [c.account_id, c])),
    rules: ["shared parent account", "majority token source", "creator / launch side"],
  };
  return { report, clusters: [...clusters, ...cleared] };
}

export const attestationPayload = (report, clusters) => JSON.stringify({
  ts: String(report.ts),
  independent_holders: report.independent_holders,
  clusters: clusters.map(({ account_id, cluster_id, size }) => ({ account_id, cluster_id, size })),
});

// ----------------------------------------------------------------------------- chain I/O
export async function allHolders(coin) {
  const { view } = await import("./lib/near.js");
  const out = [];
  for (let from = 0; ; from += 200) {
    const page = await view(coin, "get_holders", { from_index: from, limit: 200 });
    out.push(...page.map((h) => ({ account_id: h.account_id, balance: BigInt(h.balance), since_ms: Number(h.since_ms), source: h.source })));
    if (page.length < 200) return out;
  }
}

export async function score(coin, { market = [] } = {}) {
  const { view, chainNowMs, readJson } = await import("./lib/near.js");
  const [cfg, holders, creator, launchReceiver, now] = await Promise.all([
    view(coin, "get_config"), allHolders(coin), view(coin, "get_creator"), view(coin, "get_launch_receiver"), chainNowMs(),
  ]);
  const prev = readJson(path.join(here, "data", `score-${coin}.json`), { clusters: {} });
  const out = scoreHolders({ cfg, holders, creator, launchReceiver, market, now, previouslyFlagged: Object.keys(prev.clusters) });
  out.report.coin = coin;
  return out;
}

/**
 * Score, sign with `signers` oracle keys (default: the first `threshold` of them) and submit.
 * In this demo one process holds all the oracle keys; in production each signer runs this file on
 * its own machine and the relayer collects the signatures.
 */
export async function submit(coin, relayer, { dry = false, signers, market } = {}) {
  const { view, call, retry, getKey, KEY_DIR, writeJson } = await import("./lib/near.js");
  const { report, clusters } = await score(coin, { market });
  const payload = attestationPayload(report, clusters);
  const [pks, threshold] = await Promise.all([view(coin, "get_oracle_pks"), view(coin, "get_oracle_threshold")]);
  const signatures = [];
  for (const id of ORACLE_IDS) {
    const kp = await getKey(id);
    if (!kp) continue;
    const index = pks.indexOf(kp.getPublicKey().toString());
    if (index < 0 || (signers && !signers.includes(index))) continue;
    signatures.push({ index, signature: Buffer.from(kp.sign(Buffer.from(payload)).signature).toString("base64") });
    if (!signers && signatures.length >= threshold) break;
  }
  if (!signatures.length) throw new Error(`no oracle key for ${coin} in ${KEY_DIR}`);
  if (!dry) {
    // retrying is safe: if a lost response actually landed, the contract rejects the replay
    await retry(() => call(relayer, coin, "attest", { payload, signatures }, { gasTgas: 300 }))
      .catch((e) => { if (!/stale attestation/i.test(String(e.message || e))) throw e; });
    writeJson(path.join(here, "data", `score-${coin}.json`), report);
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
