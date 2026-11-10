// Example creator challenge server: the plug-in model in ~60 lines.
//
// A creator runs any game or activity. When a player completes it, this server signs
//   insydr-challenge:{coin}:{challenge_id}:{account}:{nonce}
// and the player submits that signature to the coin's `claim_signed`. The coin contract checks
// the signature against the issuer key registered with the challenge, then applies the same
// anti-farm weighting and budget caps as every other payout.
//
// usage:
//   node challenge-server.js register <coin> <creator> "<name>" <reward-tokens> <daily-budget-tokens>
//   node challenge-server.js serve <coin> [port]         POST /complete {account_id, challenge_id}
import http from "node:http";
import crypto from "node:crypto";
import { KeyPair } from "near-api-js";
import { call, view, keyStore, NETWORK, getKey, tokens } from "./lib/near.js";
import { fileURLToPath } from "node:url";

const ISSUER_ID = "issuer.local";

async function issuer() {
  let kp = await getKey(ISSUER_ID);
  if (!kp) {
    kp = KeyPair.fromRandom("ed25519");
    await keyStore.setKey(NETWORK, ISSUER_ID, kp);
  }
  return kp;
}

export async function signCompletion(coin, challengeId, accountId) {
  const nonce = crypto.randomUUID();
  const msg = `insydr-challenge:${coin}:${challengeId}:${accountId}:${nonce}`;
  const { signature } = (await issuer()).sign(Buffer.from(msg));
  return { nonce, signature: Buffer.from(signature).toString("base64") };
}

export async function register(coin, creator, name, reward, budget) {
  const pk = (await issuer()).getPublicKey().toString();
  const out = await call(creator, coin, "add_challenge", {
    name,
    proof: { Signed: { issuer_pk: pk } },
    reward: tokens(reward).toString(),
    cooldown_days: 1,
    daily_budget: tokens(budget).toString(),
  });
  const id = JSON.parse(Buffer.from(out.status.SuccessValue, "base64").toString());
  return id;
}

const isMain = process.argv[1] === fileURLToPath(import.meta.url);
const [cmd, coin, ...rest] = isMain ? process.argv.slice(2) : [];
if (cmd === "register") {
  const [creator, name, reward, budget] = rest;
  console.log("challenge id", await register(coin, creator, name, Number(reward), Number(budget)));
} else if (cmd === "serve") {
  const port = Number(rest[0] || 8787);
  http
    .createServer(async (req, res) => {
      if (req.method !== "POST" || req.url !== "/complete") return res.writeHead(404).end();
      let body = "";
      for await (const c of req) body += c;
      try {
        const { account_id, challenge_id } = JSON.parse(body);
        // A real creator checks their own game state here before signing.
        const ch = (await view(coin, "get_challenges"))[challenge_id];
        if (!ch?.active) throw new Error("unknown challenge");
        const sig = await signCompletion(coin, challenge_id, account_id);
        res.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify({ challenge_id, ...sig }));
      } catch (e) {
        res.writeHead(400).end(String(e.message || e));
      }
    })
    .listen(port, () => console.log(`challenge server for ${coin} on :${port}`));
} else if (cmd) {
  console.log("usage: register <coin> <creator> <name> <reward> <budget> | serve <coin> [port]");
}
