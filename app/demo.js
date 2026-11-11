// Public-testnet demo: the same scenario as e2e.js, in real time against NEAR testnet.
//
//   node demo.js run [--days 9] [--size full|small] [--day-min 4]   everything, resumable if interrupted
//   node demo.js report                                             who earned what
//   node demo.js lock                                               delete the coin's access keys (irreversible)
//   node demo.js new                                                forget the current coin and start a fresh one
//
// The live coin (12-hour days, locked from the first minute, kept alive by a scheduled job):
//   node demo.js live --state live                                  deploy, distribute, lock
//   node demo.js keep --state live                                  idempotent: today's attestation and check-ins, if not done yet
//
// State lives in app/data/demo.json (account ids and progress only; keys stay in app/.keys/).
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createTestnetAccount, readJson, writeJson, NETWORK } from "./lib/near.js";
import { Scenario, SIZES, fmt } from "./scenario.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const arg0 = (name, dflt) => { const i = process.argv.indexOf(`--${name}`); return i > 0 ? process.argv[i + 1] : dflt; };
const STATE = path.join(here, "data", `${arg0("state", "demo")}.json`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const arg = (name, dflt) => { const i = process.argv.indexOf(`--${name}`); return i > 0 ? process.argv[i + 1] : dflt; };

if (NETWORK !== "testnet") throw new Error("demo.js is testnet only; use e2e.js for the local sandbox");
const [cmd] = process.argv.slice(2);
let st = readJson(STATE, {});
if (cmd === "new") { st = {}; writeJson(STATE, st); log("cleared; `node demo.js run` deploys a fresh coin"); process.exit(0); }

const net = {
  name: "NEAR testnet",
  concurrency: 2, // free public RPCs rate-limit bursts
  async createTopLevel(name) {
    const id = `${name}-${Date.now().toString(36).slice(-6)}.testnet`;
    await createTestnetAccount(id); // public faucet helper: a new named account with test NEAR
    await sleep(2500);
    return id;
  },
  passTime: (ms) => sleep(Math.min(ms, 30_000)), // callers poll, so short sleeps are enough
};
const LIVE = cmd === "live" || cmd === "keep";
// the live coin: 12-hour days, and coins count after one day, so it starts within 12 hours
const liveConfig = { holder_min_days: 1, age_full_days: 1 };
const sc = new Scenario(net, st, {
  log, save: () => writeJson(STATE, st),
  sizes: SIZES[st.size ?? arg("size", "full")], dayMs: st.dayMs ?? (LIVE ? 12 * 3_600_000 : Number(arg("day-min", 4)) * 60_000),
  config: LIVE ? liveConfig : {}, seed: (Date.now() / 3_600_000) | 0,
});
st.size ??= arg("size", "full");
st.dayMs ??= sc.dayMs;

async function report() {
  const rep = await sc.report();
  sc.print(rep);
  writeJson(path.join(here, "data", "demo-report.json"), { at: new Date().toISOString(), ...rep, config: undefined });
  return rep;
}

if (cmd === "run") {
  const days = Number(arg("days", 9));
  await sc.setup();
  log(`coin page: web/index.html?coin=${st.coin}`);
  // coin-age needs real time to pass before anyone counts as a holder
  if (!st.done.includes("start")) log(`waiting ${(2 * st.dayMs) / 60000} min for coin-age (holder_min_days = 2)`);
  const cfgWait = async () => { for (let left = 2 * st.dayMs + 5_000 - (Date.now() - (st.distributedAt ?? Date.now())); left > 0; left -= 30_000) await sleep(Math.min(30_000, left)); };
  st.distributedAt ??= Date.now(); writeJson(STATE, st);
  if (!st.done.includes("start")) await cfgWait();
  await sc.start();
  st.daysRun ??= 0;
  // a normal week, one day when most real holders stay away, then recovery
  const plan = [0.9, 0.9, 0.9, 0.9, 0.9, 0.15, 0.9, 0.9, 0.9];
  while (st.daysRun < days) {
    await sc.day({ attendance: plan[st.daysRun] ?? 0.9 });
    st.daysRun += 1;
    writeJson(STATE, st);
    if (st.daysRun === 3 && !st.challenge) await sc.challenge();
  }
  await report();
} else if (cmd === "live") {
  await sc.setup();
  if (!st.locked) await sc.lock();
  log(`live coin ${st.coin}: locked. Run \`node demo.js keep --state live\` on a schedule.`);
} else if (cmd === "keep") {
  // Idempotent keeper. Safe to run as often as you like: it acts at most once per coin day.
  if (!st.coin) throw new Error("no live coin: run `node demo.js live --state live` first");
  const s = await sc.state();
  if (!s.ritual_started_ms) {
    const r = await sc.oracle();
    log(`not started yet: ${r.independent_holders} independent holders with enough coin-age (needs ${(await sc.report()).config.quorum_min})`);
  } else if (st.keptDay === s.current_day) {
    log(`day ${s.current_day} already kept`);
  } else {
    const r = await sc.oracle();
    const { organicToday, ok } = await sc.checkIns({ attendance: 0.9 });
    st.keptDay = s.current_day;
    writeJson(STATE, st);
    log(`day ${s.current_day}: ${ok.filter(Boolean).length} check-ins (${organicToday.length}/${st.organic.length} real holders), ${r.independent_holders} independent holders, streak ${s.community_streak}`);
    if (s.current_day >= 2 && !st.challenge) await sc.challenge();
    // the community grows: one new holder buys in on most days
    if (st.organic.length < 40 && Math.random() < 0.8) log(`newcomer ${(await sc.newcomer(300_000 + Math.floor(Math.random() * 6) * 60_000)).slice(0, 10)}… bought in`);
  }
} else if (cmd === "report") await report();
else if (cmd === "lock") await sc.lock();
else console.log("usage: node demo.js run [--days 9] [--size full|small] [--day-min 4] | report | lock | new | live --state live | keep --state live");
