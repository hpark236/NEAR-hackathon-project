// Shared NEAR helpers (testnet by default). Keys live in app/.keys/ (git-ignored).
import { connect, keyStores, KeyPair, utils } from "near-api-js";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const KEY_DIR = process.env.GRASSROOTS_KEY_DIR || path.join(here, "..", ".keys");
export const NETWORK = process.env.NEAR_NETWORK || "testnet";
export const RPC = process.env.NEAR_RPC || "https://test.rpc.fastnear.com";
export const TGAS = 10n ** 12n;

export const keyStore = new keyStores.UnencryptedFileSystemKeyStore(KEY_DIR);

let _near;
export async function near() {
  if (!_near) _near = await connect({ networkId: NETWORK, nodeUrl: RPC, keyStore });
  return _near;
}

export async function account(id) {
  return (await near()).account(id);
}

export async function view(contractId, methodName, args = {}) {
  const res = await fetch(RPC, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: "g",
      method: "query",
      params: {
        request_type: "call_function",
        finality: "optimistic",
        account_id: contractId,
        method_name: methodName,
        args_base64: Buffer.from(JSON.stringify(args)).toString("base64"),
      },
    }),
  });
  const j = await res.json();
  if (j.error || j.result?.error) throw new Error(JSON.stringify(j.error || j.result.error));
  return JSON.parse(Buffer.from(j.result.result).toString());
}

export async function call(signerId, contractId, methodName, args = {}, { gasTgas = 100, deposit = "0" } = {}) {
  const acc = await account(signerId);
  const out = await acc.functionCall({
    contractId,
    methodName,
    args,
    gas: BigInt(gasTgas) * TGAS,
    attachedDeposit: BigInt(deposit),
  });
  return out;
}

export async function newKey(accountId) {
  const kp = KeyPair.fromRandom("ed25519");
  await keyStore.setKey(NETWORK, accountId, kp);
  return kp;
}

export async function getKey(accountId) {
  return keyStore.getKey(NETWORK, accountId);
}

/** Create a top-level *.testnet account through the public testnet faucet helper. */
export async function createTestnetAccount(accountId) {
  const kp = await newKey(accountId);
  const res = await fetch("https://helper.nearprotocol.com/account", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ newAccountId: accountId, newAccountPublicKey: kp.getPublicKey().toString() }),
  });
  if (!res.ok) throw new Error(`faucet ${res.status}: ${await res.text()}`);
  return kp;
}

/** Create `${prefix}.${parentId}` funded with `nearAmount` NEAR. */
export async function createSubAccount(parentId, prefix, nearAmount = "0.05") {
  const id = `${prefix}.${parentId}`;
  const kp = await newKey(id);
  const parent = await account(parentId);
  await parent.createAccount(id, kp.getPublicKey(), BigInt(utils.format.parseNearAmount(nearAmount)));
  return id;
}

export const yocto = (near) => BigInt(utils.format.parseNearAmount(String(near)));
export const ONE_TOKEN = 10n ** 18n;
export const tokens = (n) => (BigInt(Math.round(n * 1000)) * ONE_TOKEN) / 1000n;
export const fmt = (raw) => (Number(BigInt(raw) / 10n ** 14n) / 1e4).toLocaleString("en-US");

export function readJson(p, fallback) {
  try {
    return JSON.parse(fs.readFileSync(p, "utf8"));
  } catch {
    return fallback;
  }
}
export function writeJson(p, v) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, JSON.stringify(v, null, 2));
}
