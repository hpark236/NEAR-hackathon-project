// The INSYDR demo scenario: one coin, one small community, three kinds of farm and the creator's
// own insiders, run day by day against the real contract.
//
// The same code drives both targets:
//   node e2e.js            local NEAR sandbox, ~2 minutes, no wallet or network needed, with assertions
//   node demo.js run       public testnet, real time (see demo.js)
//
// A `net` adapter supplies the two things that differ: how top-level accounts are created and how
// time passes.
import { KeyPair } from "near-api-js";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  account, call, view, keyStore, NETWORK, createSubAccount, yocto, tokens, fmt, getKey, retry, writeJson,
} from "./lib/near.js";
import { submit, ORACLE_IDS } from "./oracle.js";

const here = path.dirname(fileURLToPath(import.meta.url));
// a fresh local build if there is one, otherwise the committed, CI-tested binary
const BUILT = path.join(here, "..", "contracts", "insydr", "target", "near", "insydr.wasm");
export const WASM = fs.existsSync(BUILT) ? BUILT : path.join(here, "..", "contracts", "insydr", "res", "insydr.wasm");
const TAP_BUILT = path.join(here, "..", "contracts", "tap", "target", "near", "insydr_tap.wasm");
export const TAP_WASM = fs.existsSync(TAP_BUILT) ? TAP_BUILT : path.join(here, "..", "contracts", "tap", "res", "insydr_tap.wasm");
const ISSUER_ID = "issuer.local";
export const TAP_AMOUNT = 50_000; // below holder_min_balance: tap accounts never count as holders

/** Demo-scale parameters: small milestones and short days, so a coin's whole life fits in minutes. */
export const demoConfig = (dayMs) => ({
  max_supply: (1_000_000_000n * 10n ** 18n).toString(),
  launch_bps: 3500,
  pool_seed_bps: 500,
  milestones: [5, 10, 15, 40],
  tranche_bps: [500, 700, 900, 1100],
  tranche_to_pool_bps: 8000,
  contraction_days: 3,
  contraction_cut_bps: 1000,
  quorum_min: 3,
  quorum_bps: 7500,
  quorum_window_days: 5,
  daily_payout_bps: 30,
  miss_burn_bps: 100,
  age_full_days: 2,
  streak_cap: 7,
  streak_max_bonus_bps: 5000,
  streak_miss_penalty: 1,
  balance_cap_bps: 50,
  min_balance_to_check_in: tokens(1).toString(),
  season_days: 7,
  season_growth_bps: 2000,
  season_bonus_bps: 200,
  challenge_daily_cap_bps: 100,
  challenge_account_cap_bps: 2000,
  holder_min_balance: tokens(100_000).toString(),
  holder_min_days: 2,
  tranche_cooldown_days: 1,
  creator_vest_days: 30,
  day_ms: String(dayMs),
});

export const SIZES = {
  small: { organic: 8, lazy: 6, hidden: 4, patient: 3, insiders: 2, stealth: 2 },
  full: { organic: 12, lazy: 12, hidden: 6, patient: 4, insiders: 2, stealth: 2 },
};

const GROUPS = [
  ["organic", "Real holders"],
  ["lazy", "Farm, sub-accounts"],
  ["hidden", "Farm, random-looking wallets fed from one stack"],
  ["patient", "Farm, each wallet bought on its own"],
  ["insiders", "Creator's named wallets"],
  ["stealth", "Creator's unnamed wallets"],
];

function mulberry(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

async function pool(items, n, fn) {
  const queue = [...items.entries()];
  const out = new Array(items.length);
  await Promise.all(Array.from({ length: n }, async () => {
    while (queue.length) {
      const [i, x] = queue.shift();
      out[i] = await fn(x, i);
    }
  }));
  return out;
}

const panicOf = (e) => String(e?.message || e).match(/Smart contract panicked: ([^"\\]+)/)?.[1] ?? String(e?.message || e).slice(0, 160);
const result = (out) => JSON.parse(Buffer.from(out.status.SuccessValue, "base64").toString() || "null");

export class Scenario {
  /**
   * @param net   { name, createTopLevel(name, near) -> id, passTime(ms), concurrency }
   * @param st    plain object persisted by the caller (account ids and progress only; never keys)
   */
  constructor(net, st, { log = console.log, save = () => {}, sizes = SIZES.small, dayMs = 60_000, seed = 7 } = {}) {
    Object.assign(this, { net, st, log, save, sizes, dayMs });
    this.rand = mulberry(seed);
  }

  async step(name, fn) {
    this.st.done ??= [];
    if (this.st.done.includes(name)) return;
    await fn();
    this.st.done.push(name);
    this.save();
  }

  async implicit(funder, near = "0.3") {
    const kp = KeyPair.fromRandom("ed25519");
    const id = Buffer.from(kp.getPublicKey().data).toString("hex");
    await keyStore.setKey(NETWORK, id, kp);
    await retry(async () => (await account(funder)).sendMoney(id, yocto(near)));
    return id;
  }

  // ------------------------------------------------------------------ setup
  async setup() {
    const { st, net, sizes, log } = this;
    await this.step("accounts", async () => {
      st.main = await net.createTopLevel("creator");
      st.farmer = await net.createTopLevel("farmer");
      st.exchange = await net.createTopLevel("exchange"); // funds unrelated people, like a CEX withdrawal
      for (const id of [...ORACLE_IDS, ISSUER_ID]) await keyStore.setKey(NETWORK, id, KeyPair.fromRandom("ed25519"));
      log(`accounts: creator ${st.main}, farmer ${st.farmer}`);
    });
    await this.step("deploy", async () => {
      st.launch = await createSubAccount(st.main, "launch", "1");
      st.coin = await net.createTopLevel("idog", "8");
      await retry(async () => (await account(st.coin)).deployContract(fs.readFileSync(WASM)));
      const oracle_pks = [];
      for (const id of ORACLE_IDS) oracle_pks.push((await getKey(id)).getPublicKey().toString());
      await retry(() => call(st.coin, st.coin, "new", {
        name: "INSYDR Dog",
        symbol: "IDOG",
        icon: null,
        config: demoConfig(this.dayMs),
        creator: st.main,
        launch_receiver: st.launch,
        oracle_pks,
        oracle_threshold: 2,
        links: [
          { label: "source", url: "https://github.com/hpark236/NEAR-hackathon-project" },
          { label: "only official distribution", url: `the ${st.coin} contract itself` },
        ],
      }, { gasTgas: 300 }));
      log(`deployed ${st.coin}: 2-of-3 oracle, one day = ${this.dayMs / 1000}s`);
    });
    await this.step("actors", async () => {
      st.organic = [];
      for (let i = 0; i < sizes.organic; i++) st.organic.push(await this.implicit(st.exchange));
      st.lazy = [];
      for (let i = 0; i < sizes.lazy; i++) st.lazy.push(await createSubAccount(st.farmer, `f${i + 1}`, "0.3"));
      st.hidden = [];
      for (let i = 0; i < sizes.hidden; i++) st.hidden.push(await this.implicit(st.farmer));
      st.patient = [];
      for (let i = 0; i < sizes.patient; i++) st.patient.push(await this.implicit(st.exchange));
      st.insiders = [];
      for (let i = 0; i < sizes.insiders; i++) st.insiders.push(await createSubAccount(st.main, `insider${i + 1}`, "0.3"));
      st.stealth = [];
      for (let i = 0; i < sizes.stealth; i++) st.stealth.push(await this.implicit(st.main));
      log(`actors: ${GROUPS.map(([g]) => `${st[g].length} ${g}`).join(", ")}`);
    });
    await this.step("distribute", () => this.distribute());
    await this.step("tap", () => this.deployTap());
  }

  /** Demo-only faucet so a visitor can try the coin without a wallet (see contracts/tap). */
  async deployTap() {
    const { st } = this;
    st.tap = await createSubAccount(st.main, "tap", "4");
    await retry(async () => (await account(st.tap)).deployContract(fs.readFileSync(TAP_WASM)));
    await retry(() => call(st.tap, st.tap, "new", { token: st.coin, amount: tokens(TAP_AMOUNT).toString() }));
    const deposit = (await view(st.coin, "storage_balance_bounds")).min;
    await retry(() => call(st.launch, st.coin, "storage_deposit", { account_id: st.tap, registration_only: true }, { deposit }));
    await retry(async () => {
      if (BigInt(await view(st.coin, "ft_balance_of", { account_id: st.tap })) > 0n) return;
      await call(st.launch, st.coin, "ft_transfer", { receiver_id: st.tap, amount: tokens(20_000_000).toString() }, { deposit: "1" });
    });
    this.log(`demo tap ${st.tap}: ${TAP_AMOUNT.toLocaleString()} IDOG per visitor, once`);
  }

  /** Idempotent: tops each account up to its target, so an interrupted run can resume. */
  async distribute() {
    const { st } = this;
    const deposit = (await view(st.coin, "storage_balance_bounds")).min;
    const give = async (from, to, amount) => {
      const want = tokens(amount);
      if (!(await view(st.coin, "storage_balance_of", { account_id: to })))
        await retry(() => call(from, st.coin, "storage_deposit", { account_id: to, registration_only: true }, { deposit }));
      // re-check before every attempt so a lost response never pays twice
      await retry(async () => {
        const now = BigInt(await view(st.coin, "ft_balance_of", { account_id: to }));
        if (now >= want) return;
        await call(from, st.coin, "ft_transfer", { receiver_id: to, amount: (want - now).toString() }, { deposit: "1" });
      });
    };
    const L = st.launch;
    // real holders: each buys from the market, different sizes
    for (const [i, a] of st.organic.entries()) await give(L, a, 300_000 + i * 60_000);
    // the farmer buys ONE stack, then splits it across named and random-looking wallets
    const split = [...st.lazy, ...st.hidden];
    await give(L, st.farmer, 300_000 * split.length + 1_000);
    for (const a of split) await give(st.farmer, a, 300_000);
    // the patient farmer buys separately for every wallet (looks like unrelated buyers)
    for (const a of st.patient) await give(L, a, 300_000);
    // creator side: named sub-accounts, and wallets the creator feeds directly
    for (const a of st.insiders) await give(L, a, 2_000_000);
    await give(L, st.main, 2_000_000 * st.stealth.length + 1_000);
    for (const a of st.stealth) await give(st.main, a, 2_000_000);
    this.log("tokens distributed from the launch account");
  }

  // ------------------------------------------------------------------ the ritual
  get actors() {
    const { st } = this;
    return GROUPS.flatMap(([g]) => st[g].map((id) => ({ id, group: g })));
  }

  async state() {
    return view(this.st.coin, "get_state");
  }

  async oracle(opts) {
    // the tap sells to everyone, like a market: being supplied by it links nobody
    return submit(this.st.coin, this.st.exchange, { market: this.st.tap ? [this.st.tap] : [], ...opts });
  }

  /** Wait until coins have enough coin-age to count, then attest until the ritual starts. */
  async start() {
    const { st, log } = this;
    await this.step("start", async () => {
      const cfg = await view(st.coin, "get_config");
      await this.net.passTime(cfg.holder_min_days * Number(cfg.day_ms) + 2_000);
      for (let i = 0; ; i++) {
        const r = await this.oracle();
        if ((await this.state()).ritual_started_ms) {
          log(`ritual started: ${r.independent_holders} independent of ${r.holders} holders, ${Object.keys(r.clusters).length} wallets flagged`);
          break;
        }
        if (i > 20) throw new Error("ritual did not start");
        await this.net.passTime(Number(cfg.day_ms) / 4);
      }
    });
  }

  async checkIn(id) {
    try {
      await retry(() => call(id, this.st.coin, "check_in", {}, { gasTgas: 60 }));
      return true;
    } catch (e) {
      const m = panicOf(e);
      if (!/already checked in/.test(m)) this.log(`  check_in ${id.slice(0, 12)} refused: ${m}`);
      return false;
    }
  }

  /** One day: the oracle attests, people and bots check in, then the day ends. */
  async day({ attendance = 0.85 } = {}) {
    const { st, log } = this;
    const before = await this.state();
    const o = await this.oracle();
    const organicToday = st.organic.filter(() => this.rand() < attendance);
    const bots = this.actors.filter((a) => a.group !== "organic").map((a) => a.id);
    const ok = await pool([...organicToday, ...bots], this.net.concurrency ?? 4, (id) => this.checkIn(id));
    await this.nextDay(before.current_day);
    await retry(() => call(st.exchange, st.coin, "settle", {}, { gasTgas: 100 })); // anyone may settle
    const d = await view(st.coin, "get_day", { day: before.current_day });
    const s = await this.state();
    log(`day ${String(before.current_day).padStart(2)}: ${String(ok.filter(Boolean).length).padStart(2)} check-ins (${organicToday.length}/${st.organic.length} real holders)` +
      ` | turnout ${(Number(d.turnout) / 1e6).toFixed(2)} vs target ${(Number(d.target) / 1e6).toFixed(2)}` +
      ` | ${d.hit ? `HIT  paid ${fmt(d.payout)}` : `MISS burned ${fmt(d.burned)}`}` +
      ` | streak ${s.community_streak} | supply ${fmt(s.total_supply)} | milestone ${s.milestone_idx} | independent ${o.independent_holders}`);
    return { day: before.current_day, ...d, state: s, oracle: o };
  }

  async nextDay(from) {
    const dayMs = Number((await view(this.st.coin, "get_config")).day_ms);
    for (let i = 0; i < 400; i++) {
      if ((await this.state()).current_day > from) return;
      await this.net.passTime(dayMs / 4);
    }
    throw new Error("the day did not end");
  }

  // ------------------------------------------------------------------ creator challenge
  async signCompletion(challengeId, accountId) {
    const nonce = `${challengeId}-${accountId.slice(0, 8)}-${Math.floor(this.rand() * 1e9)}`;
    const msg = `insydr-challenge:${this.st.coin}:${challengeId}:${accountId}:${nonce}`;
    const { signature } = (await getKey(ISSUER_ID)).sign(Buffer.from(msg));
    return { nonce, signature: Buffer.from(signature).toString("base64") };
  }

  /** The creator plugs in a challenge; one wallet from every group completes it. */
  async challenge() {
    const { st, log } = this;
    if (st.challengeId === undefined) {
      const issuer_pk = (await getKey(ISSUER_ID)).getPublicKey().toString();
      const out = await retry(() => call(st.main, st.coin, "add_challenge", {
        name: "Daily meme raid",
        proof: { Signed: { issuer_pk } },
        reward: tokens(50_000).toString(),
        cooldown_days: 1,
        daily_budget: tokens(1_000_000).toString(),
      }));
      st.challengeId = result(out);
      this.save();
    }
    const rows = [];
    for (const [g, label] of GROUPS) {
      const who = st[g][0];
      const { nonce, signature } = await this.signCompletion(st.challengeId, who);
      let paid = 0n, note = "";
      try {
        paid = BigInt(result(await retry(() => call(who, st.coin, "claim_signed", { challenge_id: st.challengeId, nonce, signature }, { gasTgas: 60 }))));
      } catch (e) {
        note = panicOf(e);
      }
      rows.push({ group: g, label, account: who, paid: paid.toString(), note });
      log(`  challenge  ${label.padEnd(48)} ${paid ? "paid " + fmt(paid) : "refused: " + note}`);
    }
    st.challenge = rows;
    this.save();
    return rows;
  }

  // ------------------------------------------------------------------ report
  async report() {
    const { st } = this;
    const [state, cfg] = await Promise.all([this.state(), view(st.coin, "get_config")]);
    const groups = [];
    let total = 0n;
    for (const [g, label] of GROUPS) {
      let rewards = 0n, held = 0n, flagged = 0;
      for (const id of st[g]) {
        const [m, claimable, bal, cl] = await Promise.all([
          view(st.coin, "get_member", { account_id: id }), view(st.coin, "get_claimable", { account_id: id }),
          view(st.coin, "ft_balance_of", { account_id: id }), view(st.coin, "get_cluster", { account_id: id }),
        ]);
        const earned = BigInt(m?.rewards_total ?? 0) + BigInt(claimable);
        rewards += earned;
        held += BigInt(bal) - BigInt(m?.rewards_total ?? 0);
        if (cl) flagged += 1;
      }
      groups.push({ group: g, label, wallets: st[g].length, flagged, held: held.toString(), rewards: rewards.toString() });
      total += rewards;
    }
    for (const g of groups) {
      g.share_pct = total ? Number((BigInt(g.rewards) * 100000n) / total) / 1000 : 0;
      g.per_1000_tokens = BigInt(g.held) > 0n ? Number((BigInt(g.rewards) * 1_000_000n) / BigInt(g.held)) / 1000 : 0;
    }
    const n = await view(st.coin, "get_ledger_len");
    const ledger = [];
    for (let i = 0; i < n; i += 200) ledger.push(...await view(st.coin, "get_ledger", { from_index: i, limit: 200 }));
    const sum = (k) => ledger.filter((e) => e.kind === k).reduce((s, e) => s + BigInt(e.amount), 0n);
    const out = {
      coin: st.coin, network: this.net.name, groups, state, config: cfg,
      ledger: { entries: n, minted: sum("mint").toString(), burned: sum("burn").toString(), cancelled: sum("cancel").toString(),
        reconciles: sum("mint") - sum("burn") === BigInt(state.total_supply) },
      challenge: st.challenge ?? [],
    };
    return out;
  }

  print(rep) {
    const { log } = this;
    log(`\ncoin ${rep.coin} on ${rep.network}\n`);
    log("group".padEnd(50) + "wallets  flagged  rewards         share    per 1,000 coins held");
    for (const g of rep.groups) {
      log(g.label.padEnd(50) + String(g.wallets).padEnd(9) + String(g.flagged).padEnd(9) + fmt(g.rewards).padEnd(16) +
        (g.share_pct.toFixed(2) + "%").padEnd(9) + g.per_1000_tokens.toFixed(1));
    }
    const s = rep.state;
    log(`\nstreak ${s.community_streak} (best ${s.best_streak}) | supply ${fmt(s.total_supply)} | reserve ${fmt(s.reserve)} | burned ${fmt(s.burned)} | cancelled ${fmt(s.cancelled)}`);
    log(`creator earn-out: granted ${fmt(s.creator_unlocked)}, vested ${fmt(s.creator_vested)}, claimed ${fmt(s.creator_claimed)}`);
    log(`ledger: ${rep.ledger.entries} entries, minted - burned ${rep.ledger.reconciles ? "=" : "!="} total supply`);
  }

  /** Delete every full-access key of the coin account: nobody can change or redeploy it again. */
  async lock() {
    const acc = await account(this.st.coin);
    for (const k of await acc.getAccessKeys()) await retry(() => acc.deleteKey(k.public_key));
    this.st.locked = true;
    this.save();
    this.log(`deleted every access key of ${this.st.coin}: the contract is frozen`);
  }
}

export { GROUPS, panicOf, result, fmt, writeJson };
