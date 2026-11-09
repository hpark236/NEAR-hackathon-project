// End-to-end testnet demo: deploy a Grassroots coin, then run a small community through it
// alongside a sybil farm and deployer-side insiders, with the oracle attesting every "day".
//
//   node demo.js setup  <main.testnet> [--farmer <farmer.testnet>]   create actors, deploy, distribute
//   node demo.js redeploy [day_ms]                                   fresh coin for the same actors
//   node demo.js distribute                                          resume token distribution after a crash
//   node demo.js fund [near]                                         top actors up so they can pay gas
//   node demo.js run    [days]                                       run the ritual for N days
//   node demo.js challenge                                           creator plugs in a signed challenge; actors claim
//   node demo.js report                                              who earned what
//   node demo.js lock                                                delete the coin's full-access key
//
// State lives in app/data/demo.json (account ids only; keys stay in app/.keys/).
import { KeyPair } from "near-api-js";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  account, call, view, keyStore, NETWORK, createTestnetAccount, createSubAccount,
  yocto, tokens, fmt, readJson, writeJson, getKey, retry,
} from "./lib/near.js";
import { submit } from "./oracle.js";
import { register as registerChallenge, signCompletion } from "./challenge-server.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const STATE = path.join(here, "data", "demo.json");
const WASM = path.join(here, "..", "contracts", "grassroots", "target", "near", "grassroots.wasm");
const st = readJson(STATE, {});
const save = () => writeJson(STATE, st);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

const N_ORGANIC = 12;
const N_FARM_CAUGHT = 24;     // sub-accounts of the farmer: caught by the shared-parent rule
const N_FARM_HIDDEN = 6;      // implicit accounts funded by the farmer: NOT caught by v1
const N_INSIDERS = 4;         // creator-side wallets

// Demo-scale parameters: one "day" is 4 minutes and milestones are small, so the whole life
// of a coin fits in a few minutes. Production values are in sim/README.md.
const DEMO_CONFIG = {
  max_supply: (1_000_000_000n * 10n ** 18n).toString(),
  launch_bps: 3500,
  pool_seed_bps: 500,
  milestones: [5, 10, 15, 25],
  tranche_bps: [500, 700, 900, 1100],
  tranche_to_pool_bps: 8000,
  contraction_days: 3,
  contraction_cut_bps: 1000,
  quorum_min: 3,
  quorum_bps: 5000,
  daily_payout_bps: 300,
  miss_burn_bps: 100,
  age_full_days: 2,
  streak_cap: 7,
  streak_max_bonus_bps: 5000,
  balance_cap_bps: 5,
  min_balance_to_check_in: tokens(1).toString(),
  season_days: 7,
  season_growth_bps: 5000,
  season_bonus_bps: 200,
  challenge_daily_cap_bps: 100,
  day_ms: "240000",
};

async function implicitAccount(funder, label) {
  const kp = KeyPair.fromRandom("ed25519");
  const id = Buffer.from(kp.getPublicKey().data).toString("hex");
  await keyStore.setKey(NETWORK, id, kp);
  await (await account(funder)).sendMoney(id, yocto("0.05"));
  log(`  ${label} -> ${id.slice(0, 10)}… (implicit)`);
  return id;
}

async function setup(main, farmer) {
  if (!(await getKey(main))) {
    log(`creating ${main} via testnet faucet`);
    await createTestnetAccount(main);
  }
  if (!farmer) farmer = `grfarm-${Date.now().toString(36)}.testnet`;
  if (!(await getKey(farmer))) {
    log(`creating farmer ${farmer} via testnet faucet`);
    await createTestnetAccount(farmer);
  }
  if (!(await getKey("oracle.local"))) await keyStore.setKey(NETWORK, "oracle.local", KeyPair.fromRandom("ed25519"));
  const oraclePk = (await getKey("oracle.local")).getPublicKey().toString();

  Object.assign(st, { main, farmer, organic: [], farmCaught: [], farmHidden: [], insiders: [] });
  save();

  log("creating actors");
  for (let i = 0; i < N_ORGANIC; i++) st.organic.push(await implicitAccount(main, `organic ${i + 1}`));
  save();
  for (let i = 0; i < N_FARM_CAUGHT; i++) st.farmCaught.push(await createSubAccount(farmer, `f${i + 1}`, "0.05"));
  for (let i = 0; i < N_FARM_HIDDEN; i++) st.farmHidden.push(await implicitAccount(farmer, `hidden farm ${i + 1}`));
  save();
  for (let i = 0; i < N_INSIDERS; i++) st.insiders.push(await createSubAccount(main, `insider${i + 1}`, "0.05"));
  save();

  // deploy last: the coin's clock (days, seasons) starts at `new`, so start the run right after
  await deployCoin();
  await distribute();
  await fund();
}

/**
 * Deploy a fresh coin for the existing actors. The coin gets its own faucet-funded top-level
 * account (the wasm needs ~4.5 NEAR of storage); the launch account is a sub-account of main.
 */
async function deployCoin(dayMs = DEMO_CONFIG.day_ms) {
  const { main } = st;
  if (!(await getKey("oracle.local"))) await keyStore.setKey(NETWORK, "oracle.local", KeyPair.fromRandom("ed25519"));
  const oraclePk = (await getKey("oracle.local")).getPublicKey().toString();
  const tag = Date.now().toString(36).slice(-5);
  const coin = `gdog-${tag}.testnet`;
  const launch = await createSubAccount(main, `launch${tag}`, "0.5");
  log(`creating ${coin} via testnet faucet and deploying`);
  await createTestnetAccount(coin);
  await sleep(3000);
  await retry(async () => (await account(coin)).deployContract(fs.readFileSync(WASM)));
  await retry(() => call(coin, coin, "new", {
    name: "Grassroots Dog",
    symbol: "GDOG",
    icon: null,
    config: { ...DEMO_CONFIG, day_ms: String(dayMs) },
    creator: main,
    launch_receiver: launch,
    oracle_pk: oraclePk,
    links: [
      { label: "repo", url: "https://github.com/hpark236/NEAR-hackathon-project" },
      { label: "only official distribution", url: `the ${coin} contract itself` },
    ],
  }, { gasTgas: 300 }));
  Object.assign(st, { coin, launch });
  delete st.challengeId;
  save();
  log(`deployed ${coin} (one day = ${Number(dayMs) / 1000}s)`);
}

/** Idempotent: tops each actor up to its target balance, so a crashed setup can be resumed. */
async function distribute() {
  const { coin, launch } = st;
  log("registering storage and distributing tokens from the launch account");
  const deposit = (await view(coin, "storage_balance_bounds")).min;
  const give = async (to, amount) => {
    const want = tokens(amount);
    const have = BigInt((await view(coin, "ft_balance_of", { account_id: to })) || "0");
    if (have >= want) return;
    if (!(await view(coin, "storage_balance_of", { account_id: to })))
      await retry(() => call(launch, coin, "storage_deposit", { account_id: to, registration_only: true }, { deposit }));
    // re-check before retrying a transfer so a lost response never pays twice
    await retry(async () => {
      const now = BigInt((await view(coin, "ft_balance_of", { account_id: to })) || "0");
      if (now >= want) return;
      await call(launch, coin, "ft_transfer", { receiver_id: to, amount: (want - now).toString() }, { deposit: "1" });
    });
  };
  // organic: varied stacks; the farm splits ONE stack across 30 wallets; insiders hold big bags
  for (const [i, a] of st.organic.entries()) await give(a, 20_000 + i * 7_000);
  const farmStack = 600_000;
  for (const a of [...st.farmCaught, ...st.farmHidden]) await give(a, farmStack / (N_FARM_CAUGHT + N_FARM_HIDDEN));
  for (const a of st.insiders) await give(a, 400_000);
  save();
  log(`setup done. coin = ${coin}`);
}

/** Top every actor up to `target` NEAR so it can pay gas (prepaid gas is reserved up front). */
async function fund(target = "0.2") {
  const want = BigInt(yocto(target));
  const groups = [[st.main, [...st.organic, ...st.insiders]], [st.farmer, [...st.farmCaught, ...st.farmHidden]]];
  for (const [funder, ids] of groups) {
    const f = await account(funder);
    for (const id of ids) {
      const bal = BigInt((await retry(async () => (await account(id)).getAccountBalance())).available);
      if (bal < want) await retry(() => f.sendMoney(id, want - bal));
    }
  }
  log(`actors funded to ${target} NEAR`);
}

async function day(dayIdx) {
  // organic people show up most days but not all; farm and insiders script a perfect streak
  const tryCheckIn = async (a) => {
    try {
      await retry(() => call(a, st.coin, "check_in", {}, { gasTgas: 50 }));
      return true;
    } catch (e) {
      const m = String(e.message || e);
      if (!m.includes("already checked in")) log(`  check_in ${a.slice(0, 14)} failed: ${m.slice(0, 120)}`);
      return false;
    }
  };
  const organicToday = st.organic.filter(() => Math.random() < 0.8);
  const bots = [...st.farmCaught, ...st.farmHidden, ...st.insiders];
  // a few at a time: the free public RPC rate-limits bursts
  const queue = [...organicToday, ...bots];
  const res = [];
  await Promise.all(Array.from({ length: 2 }, async () => {
    while (queue.length) res.push(await tryCheckIn(queue.shift()));
  }));
  log(`day ${dayIdx}: ${res.filter(Boolean).length} check-ins (${organicToday.length} organic, ${bots.length} scripted)`);
}

async function run(days) {
  const cfg = await view(st.coin, "get_config");
  const dayMs = Number(cfg.day_ms);
  for (let d = 0; d < days; d++) {
    const r = await submit(st.coin, st.main);
    log(`oracle: ${r.independent_holders} independent of ${r.holders} holders; ${Object.keys(r.clusters).length} flagged`);
    const s0 = await view(st.coin, "get_state");
    await day(s0.current_day);
    const s = await view(st.coin, "get_state");
    log(`  supply ${fmt(s.total_supply)} | pool ${fmt(s.pool_free)} | reserve ${fmt(s.reserve)} | streak ${s.community_streak} | milestone ${s.milestone_idx}`);
    // wait for the contract's next day boundary
    while ((await view(st.coin, "get_state")).current_day <= s0.current_day) await sleep(Math.min(5_000, dayMs / 6));
  }
}

async function report() {
  const groups = { organic: st.organic, "farm (caught)": st.farmCaught, "farm (hidden)": st.farmHidden, insiders: st.insiders };
  const out = {};
  let total = 0n;
  for (const [g, ids] of Object.entries(groups)) {
    let r = 0n;
    for (const id of ids) {
      const m = await view(st.coin, "get_member", { account_id: id });
      if (m) r += BigInt(m.rewards_total);
    }
    out[g] = { accounts: ids.length, rewards: r };
    total += r;
  }
  const state = await view(st.coin, "get_state");
  console.log(`\ncoin ${st.coin}\n`);
  console.log("group            accounts   rewards           share");
  for (const [g, v] of Object.entries(out)) {
    const share = total ? Number((v.rewards * 10000n) / total) / 100 : 0;
    console.log(`${g.padEnd(17)}${String(v.accounts).padEnd(11)}${fmt(v.rewards).padEnd(18)}${share}%`);
  }
  console.log("\nstate", { ...state, total_supply: fmt(state.total_supply), reserve: fmt(state.reserve), pool_free: fmt(state.pool_free), burned: fmt(state.burned), cancelled: fmt(state.cancelled) });
  writeJson(path.join(here, "data", "demo-report.json"), { coin: st.coin, at: new Date().toISOString(), groups: Object.fromEntries(Object.entries(out).map(([g, v]) => [g, { ...v, rewards: v.rewards.toString() }])), state });
}

/** Creator plugs in a challenge ("meme raid"), then a few people and farm wallets complete it. */
async function challenge() {
  if (st.challengeId === undefined) {
    st.challengeId = await registerChallenge(st.coin, st.main, "Daily meme raid", 2_000, 50_000);
    save();
    log(`registered challenge ${st.challengeId}`);
  }
  const claim = async (who, label) => {
    const { nonce, signature } = await signCompletion(st.coin, st.challengeId, who);
    try {
      const out = await call(who, st.coin, "claim_signed", { challenge_id: st.challengeId, nonce, signature }, { gasTgas: 50 });
      const paid = JSON.parse(Buffer.from(out.status.SuccessValue, "base64").toString());
      log(`  ${label.padEnd(14)} ${who.slice(0, 18).padEnd(19)} paid ${fmt(paid)}`);
    } catch (e) {
      log(`  ${label.padEnd(14)} ${who.slice(0, 18).padEnd(19)} refused: ${String(e.message || e).match(/Smart contract panicked: ([^"\\]+)/)?.[1] || e.message}`);
    }
  };
  for (const a of st.organic.slice(0, 3)) await claim(a, "organic");
  for (const a of st.farmCaught.slice(0, 3)) await claim(a, "farm (caught)");
  for (const a of st.farmHidden.slice(0, 2)) await claim(a, "farm (hidden)");
  for (const a of st.insiders.slice(0, 2)) await claim(a, "insider");
}

async function lock() {
  const acc = await account(st.coin);
  const keys = await acc.getAccessKeys();
  for (const k of keys) {
    if (k.access_key.permission === "FullAccess") {
      await acc.deleteKey(k.public_key);
      log(`deleted full-access key ${k.public_key} from ${st.coin}`);
    }
  }
  log("contract is now frozen: no key can redeploy or change it");
}

const [cmd, ...args] = process.argv.slice(2);
if (cmd === "setup") {
  const fi = args.indexOf("--farmer");
  await setup(args[0], fi >= 0 ? args[fi + 1] : undefined);
} else if (cmd === "run") await run(Number(args[0] || 6));
else if (cmd === "redeploy") {
  await deployCoin(args[0] || DEMO_CONFIG.day_ms);
  await distribute();
  await fund();
} else if (cmd === "distribute") await distribute();
else if (cmd === "fund") await fund(args[0]);
else if (cmd === "report") await report();
else if (cmd === "challenge") await challenge();
else if (cmd === "lock") await lock();
else console.log("usage: node demo.js setup <main.testnet> [--farmer x.testnet] | redeploy [day_ms] | distribute | fund | run [days] | challenge | report | lock");
