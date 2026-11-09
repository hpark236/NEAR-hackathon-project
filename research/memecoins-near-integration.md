# Memecoins on NEAR: what NEAR does and how to integrate

Researched 2026-11-09. Sources:
- Memecoins track prompt (`nearai/near-hackathon-prompt-examples/tracks/11-memecoins.md`)
- NEP-141 spec
- meme.cooking docs and its public API (`https://api.meme.cooking/doc`)
- meme.cooking source (`github.com/Shitzu-Apes/launchpad`, forked from Skyward Finance)

## 1. What NEAR's job is in a memecoin

The **token is a smart contract**, and NEAR runs it. That contract is the only place the coin "exists." It keeps the balance table, and every transfer, mint or burn is a call to it.

For this track, NEAR's role is to be the **referee nobody can switch off**:

| Without NEAR (a normal app) | With the mechanic in a NEAR contract |
|---|---|
| The rules live in a backend the dev can change or turn off | The rules live in contract code anyone can read; with no admin key, nobody can change them |
| "Trust me, I burned it" | Every mint and burn is a public on-chain receipt |
| Supply lives in a database | Supply is the contract's `ft_total_supply()`, which anyone can query |

The organizers say it directly: *"the mechanic in the contract, not in a backend somebody could switch off"* and *"no admin key that can change [parameters] later."* Judges score the **mechanic**, not the price.

### NEAR-specific features that matter

- **NEP-141:** NEAR's fungible-token standard, its equivalent of ERC-20. It defines `ft_transfer`, `ft_transfer_call`, `ft_balance_of`, `ft_total_supply` and `ft_metadata`. Wallets, Rhea/Ref and NEAR Intents all understand any NEP-141 token. The standard has no built-in mint or burn; **your contract adds those**, which is exactly where the "mechanic" goes.
- **Named accounts:** the token gets an address like `streak.yourname.near` instead of `0x…`. Sub-accounts (`treasury.streak.near`) make the structure readable.
- **Storage staking:** each holder's balance costs a little NEAR in storage (`storage_deposit`). Keep it in mind for check-in or holder lists, and make the user pay their own storage.
- **Locking the contract:** after deploying, delete the full-access keys. The code then can't be upgraded and the parameters are final. This is the "no admin key" proof.
- **Function-call keys / gasless (NEP-366 meta-transactions):** optional. Users can check in without holding NEAR for gas, because a relayer pays.
- **Testnet is fine** for this track (rule Q-010). Intents is the only mainnet-only part.

## 2. What meme.cooking actually does (checked against its API and code)

meme.cooking is NEAR's memecoin launchpad and an official partner. It has **0 integrations** among the 336 submissions so far.

- A creator sets name, ticker, logo, an **auction duration (5 min to 1 day)**, a **soft cap** (minimum to launch), an optional hard cap and an optional vested team allocation.
- It runs a **fair auction**: everyone deposits wNEAR and everyone gets the same price. It isn't a bonding curve.
- If the soft cap is met, it **deploys the token as a sub-account of `meme-cooking.near`** (e.g. `neardog-0.meme-cooking.near`). It puts all deposited NEAR plus tokens into a **Ref/Rhea pool and locks the LP**.
- Observed defaults: **1,000,000,000 supply, 18 decimals**, a soft cap of about **100 NEAR** on recent launches, and deposits in `wrap.near`. Only **267 of 2,319** launches reached their cap.
- It has a public read API (`api.meme.cooking`) with `/meme`, `/meme/{id}`, `/trades`, OHLC data and `/profile/{account}`.

**The catch:** meme.cooking deploys a **standard, fixed-supply token from its factory**. You **cannot put custom mint or burn logic inside a meme.cooking token.** That shapes the architecture.

## 3. Two ways to build it

### A) Own token contract (mechanic inside the token)
Write a NEP-141 contract in Rust with `near-sdk` that adds the mechanic, such as daily check-in, conditional mint and treasury burn. Deploy it, then delete its keys.
- ✅ The mechanic changes supply directly. This matches the prompt exactly ("NEP-141 token with the mechanic in the contract").
- ❌ No meme.cooking launch. Liquidity would need a separate Rhea pool, and it's optional for judging.

### B) meme.cooking token + mechanic contract (mechanic around the token)
Launch a normal token on meme.cooking. A separate "ritual" contract holds a **treasury of that token** and runs the rules. Rewards pay out from the treasury, and "burns" send treasury tokens to a dead account nobody controls. The contract receives tokens through `ft_transfer_call` → `ft_on_transfer`.
- ✅ Real partner integration (meme.cooking + Rhea pool), the first in the hackathon. Anyone can plug the mechanic into an existing meme.
- ❌ It can't mint, so it's limited to "emissions from a pre-funded treasury." A mainnet launch needs about 100 NEAR in soft-cap deposits, so a demo realistically runs on testnet with a mocked launch.

### C) Hybrid (recommended)
Write the mechanic as a contract that works with **any** NEP-141 token. Demo it on testnet with your own token (path A). Add a meme.cooking adapter that reads a launched meme from `api.meme.cooking` and attaches the ritual treasury to it (path B). The judges see both the full mechanic and the partner tie-in.

## 4. Where this leaves the idea shortlist

1. **Streak coin** (the organizers' example, unbuilt). Path A, or C for the meme.cooking hook.
2. **Insider-proof launch.** On-chain wallet clustering (from the `ihaterugs` work) flags sniper or insider wallets; the token or ritual contract vests or taxes them. An off-chain clustering job publishes a signed list; the contract enforces it.
   - Honest weakness: a trusted list signer. To reduce it, publish the evidence, add a challenge window, or run the scorer in a NEAR AI Cloud TEE so its output is attested.

## 5. Open questions to settle before building
- Testnet only, or a small mainnet meme.cooking launch (~100 NEAR of deposits needed)?
- Sybil resistance for check-ins: one check-in per account is trivially farmed with many accounts. Options: a minimum holding/age, a check-in fee, or a stake. The prompt says to state the limits plainly.
- Per the prompt, simulate 60 days of community behavior before fixing parameters.
