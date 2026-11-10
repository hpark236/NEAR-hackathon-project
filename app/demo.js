// Public-testnet demo: the same scenario as e2e.js, in real time against NEAR testnet.
//
//   node demo.js run [--days 9] [--size full|small] [--day-min 4]   everything, resumable if interrupted
//   node demo.js report                                             who earned what
//   node demo.js lock                                               delete the coin's access keys (irreversible)
//   node demo.js new                                                forget the current coin and start a fresh one
//
// State lives in app/data/demo.json (account ids and progress only; keys stay in app/.keys/).
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createTestnetAccount, readJson, writeJson, NETWORK } from "./lib/near.js";
import { Scenario, SIZES, fmt } from "./scenario.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const STATE = path.join(here, "data", "demo.json");
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
const sc = new Scenario(net, st, {
  log, save: () => writeJson(STATE, st),
  sizes: SIZES[st.size ?? arg("size", "full")], dayMs: st.dayMs ?? Number(arg("day-min", 4)) * 60_000,
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
} else if (cmd === "report") await report();
else if (cmd === "lock") await sc.lock();
else console.log("usage: node demo.js run [--days 9] [--size full|small] [--day-min 4] | report | lock | new");
