// End-to-end check on a local NEAR sandbox: the real compiled contract, real signed transactions,
// no wallet, no testnet, no funds. About two minutes.
//
//   cd contracts/insydr && cargo near build non-reproducible-wasm     (or use the committed wasm, see README)
//   cd app && npm install && npm run e2e
//
// It deploys a coin, runs a small community next to three kinds of wallet farm and the creator's
// insiders for ten days, then asserts every claim the README makes. Exit code 0 = all passed.
import os from "node:os";
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { Sandbox, DEFAULT_ACCOUNT_ID, DEFAULT_PRIVATE_KEY } from "near-sandbox";

const here = path.dirname(fileURLToPath(import.meta.url));
const keyDir = fs.mkdtempSync(path.join(os.tmpdir(), "insydr-e2e-keys-"));
const sandbox = await Sandbox.start({});
process.env.NEAR_RPC = sandbox.rpcUrl;
process.env.NEAR_NETWORK = "sandbox";
process.env.INSYDR_KEY_DIR = keyDir;

const { KeyPair } = await import("near-api-js");
const near = await import("./lib/near.js");
const { Scenario, SIZES, WASM, fmt, panicOf } = await import("./scenario.js");
const { attestationPayload, score, ORACLE_IDS } = await import("./oracle.js");
const { account, call, view, keyStore, rpc, getKey, yocto } = near;

const results = [];
const check = async (name, fn) => {
  try {
    await fn();
    results.push([true, name]);
    console.log(`  PASS  ${name}`);
  } catch (e) {
    results.push([false, name, e.message]);
    console.log(`  FAIL  ${name}\n        ${String(e.message).split("\n")[0]}`);
  }
};
const rejects = async (fn, pattern) => {
  try {
    await fn();
  } catch (e) {
    assert.match(panicOf(e), pattern);
    return;
  }
  assert.fail(`expected a rejection matching ${pattern}`);
};

let failed = false;
try {
  if (!fs.existsSync(WASM)) throw new Error(`build the contract first: ${WASM} not found`);
  await keyStore.setKey("sandbox", DEFAULT_ACCOUNT_ID, KeyPair.fromString(DEFAULT_PRIVATE_KEY));
  const net = {
    name: "local sandbox",
    concurrency: 6,
    async createTopLevel(name, nearAmount = "300") {
      const id = `${name}.${DEFAULT_ACCOUNT_ID}`;
      const kp = KeyPair.fromRandom("ed25519");
      await keyStore.setKey("sandbox", id, kp);
      await (await account(DEFAULT_ACCOUNT_ID)).createAccount(id, kp.getPublicKey(), yocto(nearAmount));
      return id;
    },
    // the sandbox can fast-forward its own clock: ~0.3 s of chain time per skipped block
    async passTime(ms) {
      await rpc("sandbox_fast_forward", { delta_height: Math.ceil(ms / 290) + 2 });
    },
  };
  const st = {};
  const sc = new Scenario(net, st, { sizes: SIZES.small, dayMs: 60_000 });
  const t0 = Date.now();

  console.log("\n== setup ==");
  await sc.setup();
  const coin = st.coin;

  console.log("\n== before the ritual starts ==");
  await check("nobody can check in before the community exists", () =>
    rejects(() => call(st.organic[0], coin, "check_in", {}), /ritual has not started/));
  await check("the contract records where each wallet's coins came from", async () => {
    assert.equal(await view(coin, "get_source", { account_id: st.hidden[0] }), st.farmer);
    assert.equal(await view(coin, "get_source", { account_id: st.organic[0] }), st.launch);
    assert.equal(await view(coin, "get_source", { account_id: st.stealth[0] }), st.main);
  });
  await check("a forged oracle attestation is rejected", async () => {
    const { report, clusters } = await score(coin);
    report.independent_holders = 40;
    const payload = attestationPayload(report, clusters);
    const sig = (kp) => Buffer.from(kp.sign(Buffer.from(payload)).signature).toString("base64");
    const fake = KeyPair.fromRandom("ed25519");
    await rejects(() => call(st.exchange, coin, "attest", { payload, signatures: [{ index: 0, signature: sig(fake) }, { index: 1, signature: sig(fake) }] }, { gasTgas: 300 }),
      /bad oracle signature/);
  });
  await check("one oracle key alone cannot attest (2 of 3 required)", async () => {
    const { report, clusters } = await score(coin);
    const payload = attestationPayload(report, clusters);
    const kp = await getKey(ORACLE_IDS[0]);
    const pks = await view(coin, "get_oracle_pks");
    const signatures = [{ index: pks.indexOf(kp.getPublicKey().toString()), signature: Buffer.from(kp.sign(Buffer.from(payload)).signature).toString("base64") }];
    await rejects(() => call(st.exchange, coin, "attest", { payload, signatures }, { gasTgas: 300 }), /not enough oracle signatures/);
  });
  await check("the oracle cannot report more holders than exist on-chain", async () => {
    const { report, clusters } = await score(coin);
    report.independent_holders = 1_000_000;
    const payload = attestationPayload(report, clusters);
    const pks = await view(coin, "get_oracle_pks");
    const signatures = [];
    for (const id of ORACLE_IDS.slice(0, 2)) {
      const kp = await getKey(id);
      signatures.push({ index: pks.indexOf(kp.getPublicKey().toString()), signature: Buffer.from(kp.sign(Buffer.from(payload)).signature).toString("base64") });
    }
    await rejects(() => call(st.exchange, coin, "attest", { payload, signatures }, { gasTgas: 300 }), /cannot exceed on-chain holders/);
  });

  console.log("\n== nine days ==");
  await sc.start();
  const days = [];
  // days 0-4: a normal week. day 5: most real holders stay away. days 6-8: they come back.
  for (const attendance of [0.9, 0.9, 0.9, 0.9, 0.9, 0.15, 0.9, 0.9, 0.9]) {
    days.push(await sc.day({ attendance }));
    if (days.length === 3) await sc.challenge();
  }

  console.log("\n== a visitor tries it ==");
  let guest;
  await check("a visitor with no coins gets 50,000 from the demo tap, once", async () => {
    guest = await net.createTopLevel("guest", "5");
    await call(guest, st.tap, "drip", {}, { gasTgas: 100 });
    assert.equal(await view(coin, "ft_balance_of", { account_id: guest }), near.tokens(50_000).toString());
    await rejects(() => call(guest, st.tap, "drip", {}, { gasTgas: 100 }), /already used the tap/);
  });
  await check("the visitor can check in at once, but fresh coins weigh nothing yet", async () => {
    await call(guest, coin, "check_in", {}, { gasTgas: 60 });
    // full weight for 50,000 coins against a 5,000,000 cap is 10,000 (scale 1e6); seconds-old coins are near zero
    const w = Number((await view(coin, "get_member", { account_id: guest })).pending[0][1]);
    assert.ok(w < 500, `weight ${w}`);
  });
  await check("a day later the visitor's weight has ramped up, and they are not flagged", async () => {
    days.push(await sc.day({ attendance: 0.9 }));
    assert.ok(Number(await view(coin, "preview_weight", { account_id: guest })) > 2_000);
    assert.equal(await view(coin, "get_cluster", { account_id: guest }), null);
  });
  await check("tap accounts never count as independent holders, so they cannot unlock supply", async () => {
    await sc.net.passTime(3 * 60_000);
    const r = await sc.oracle();
    assert.equal(r.independent_holders, 12);
    assert.equal(r.clusters[st.tap]?.cluster_id, "deployer", "the tap itself is creator side");
  });

  console.log("\n== results ==");
  const rep = await sc.report();
  sc.print(rep);
  const G = Object.fromEntries(rep.groups.map((g) => [g.group, g]));
  const flags = async (ids) => (await Promise.all(ids.map((id) => view(coin, "get_cluster", { account_id: id })))).filter(Boolean).length;

  console.log("\n== checks ==");
  await check("supply grew only through milestones, one tranche per day", async () => {
    assert.equal(rep.state.milestone_idx, 2, "12 independent holders: milestones 5 and 10, not 15");
    const idx = days.map((d) => d.state.milestone_idx);
    for (let i = 1; i < idx.length; i++) assert.ok(idx[i] - idx[i - 1] <= 1, `two tranches on one day: ${idx}`);
  });
  await check("the sub-account farm and the split-stack farm are flagged; real holders are not", async () => {
    assert.equal(await flags(st.lazy), st.lazy.length);
    assert.equal(await flags(st.hidden), st.hidden.length);
    assert.equal(await flags(st.organic), 0);
  });
  await check("both kinds of creator wallet are flagged as creator side", async () => {
    for (const id of [...st.insiders, ...st.stealth]) assert.equal((await view(coin, "get_cluster", { account_id: id }))?.id, "deployer");
  });
  await check("flagged farm wallets earn under 10% of what real holders earn per coin", async () => {
    assert.ok(G.organic.per_1000_tokens > 0);
    for (const g of ["lazy", "hidden"]) assert.ok(G[g].per_1000_tokens < G.organic.per_1000_tokens * 0.1, `${g}: ${G[g].per_1000_tokens} vs ${G.organic.per_1000_tokens}`);
  });
  await check("creator-side wallets earn under 1% of what real holders earn per coin", async () => {
    for (const g of ["insiders", "stealth"]) assert.ok(G[g].per_1000_tokens < G.organic.per_1000_tokens * 0.01, `${g}: ${G[g].per_1000_tokens}`);
  });
  await check("the undetected farm earns no more than its coins and attendance explain (under 1.6x a real holder)", async () => {
    const ratio = G.patient.per_1000_tokens / G.organic.per_1000_tokens;
    assert.ok(ratio > 0.5 && ratio < 1.6, `ratio ${ratio.toFixed(2)}`);
  });
  await check("the day most real holders stayed away was a miss: pool burned, streak reset", async () => {
    const bad = days[5];
    assert.equal(bad.hit, false);
    assert.ok(BigInt(bad.burned) > 0n);
    assert.equal(bad.state.community_streak, 0);
    assert.ok(days[4].hit && days[4].state.community_streak >= 4, "the week before was a streak");
  });
  await check("the target came back down and the streak restarted", async () => {
    assert.ok(days[8].hit && days[8].state.community_streak >= 3);
  });
  await check("the challenge paid by stake and independence, not per wallet", async () => {
    const paid = Object.fromEntries(rep.challenge.map((c) => [c.group, BigInt(c.paid)]));
    assert.ok(paid.organic > 0n);
    assert.ok(paid.lazy * 5n < paid.organic && paid.hidden * 5n < paid.organic, "flagged farm wallets get a cluster share");
    assert.ok(paid.insiders * 100n < paid.organic && paid.stealth * 100n < paid.organic, "creator side gets dust");
  });
  await check("the creator's cut is vesting, not paid out", async () => {
    const s = rep.state;
    assert.ok(BigInt(s.creator_unlocked) > 0n);
    assert.ok(BigInt(s.creator_vested) > 0n && BigInt(s.creator_vested) < BigInt(s.creator_unlocked) / 2n, "about a quarter of 30 days has passed");
    assert.equal(s.creator_claimed, "0");
  });
  await check("the on-chain ledger reconciles with total supply", async () => {
    assert.ok(rep.ledger.reconciles);
    assert.equal(rep.ledger.burned, rep.state.burned);
    assert.equal(rep.ledger.cancelled, rep.state.cancelled);
  });
  await check("every token is accounted for: supply + burned + reserve + cancelled + creator's unclaimed = max", async () => {
    const s = rep.state;
    const sum = BigInt(s.total_supply) + BigInt(s.burned) + BigInt(s.reserve) + BigInt(s.cancelled) + BigInt(s.creator_unlocked) - BigInt(s.creator_claimed);
    assert.equal(sum, BigInt(rep.config.max_supply));
  });
  await check("a flagged insider can still sell everything (never a honeypot)", async () => {
    const who = st.insiders[0];
    const bal = await view(coin, "ft_balance_of", { account_id: who });
    await call(who, coin, "ft_transfer", { receiver_id: st.exchange, amount: bal }, { deposit: "1" }).catch(async (e) => {
      // the exchange account needs storage first
      const deposit = (await view(coin, "storage_balance_bounds")).min;
      await call(who, coin, "storage_deposit", { account_id: st.exchange, registration_only: true }, { deposit });
      await call(who, coin, "ft_transfer", { receiver_id: st.exchange, amount: bal }, { deposit: "1" });
    });
    assert.equal(await view(coin, "ft_balance_of", { account_id: who }), "0");
  });
  await check("after the keys are deleted nobody can redeploy or reconfigure the coin", async () => {
    await sc.lock();
    assert.equal((await (await account(coin)).getAccessKeys()).length, 0);
    await assert.rejects(async () => (await account(coin)).deployContract(fs.readFileSync(WASM)));
    // ...and the coin keeps working
    await call(st.organic[0], coin, "claim", {}, { gasTgas: 60 });
  });

  const passed = results.filter((r) => r[0]).length;
  failed = passed !== results.length;
  console.log(`\n${passed}/${results.length} checks passed in ${Math.round((Date.now() - t0) / 1000)} s on a local NEAR sandbox`);
  near.writeJson(path.join(here, "data", "e2e-report.json"), {
    at: new Date().toISOString(), checks: results.map(([ok, name, err]) => ({ ok, name, ...(err ? { err } : {}) })),
    groups: rep.groups, state: rep.state, ledger: rep.ledger, challenge: rep.challenge,
    days: days.map((d) => ({ day: d.day, hit: d.hit, checkins: d.checkins, unique: d.unique, turnout: d.turnout, target: d.target, payout: d.payout, burned: d.burned })),
  });
} catch (e) {
  failed = true;
  console.error("\nE2E aborted:", e);
} finally {
  await sandbox.tearDown();
  fs.rmSync(keyDir, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
