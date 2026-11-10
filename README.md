# Grassroots

**Memecoins that can only grow with their real community.** Built at NEAR HACKATHON (Nov 2026).
Tracks: Memecoins With Mechanics (primary), Social Money, AI Security & Defense.

**In 30 seconds:** holders check in once a day. If enough *independent* people show up, the community pool pays that day's check-ins and the streak grows; if not, a slice burns and the streak resets. New supply is minted only when the number of independent holders crosses a milestone, and wallet farms, split stacks and creator-side insiders earn almost nothing. All of it lives in the NEP-141 contract, and the parameters are fixed at launch.

### Against the official track brief ([`research/official/11-memecoins.md`](research/official/11-memecoins.md))

| The brief asks for | Where it is |
|---|---|
| NEP-141 token with the mechanic in the contract, not a backend somebody could switch off | `contracts/grassroots/src/lib.rs`. Check-in, quorum, payout, burn, streak, seasons and challenges all run on-chain. The only off-chain piece is the holder-count oracle: if it stops, the daily ritual keeps working; only new growth tranches and new farm flags pause. |
| Parameters (target, mint size, burn size) set at launch, readable by anyone, no admin key | `Config` is passed to `new` and has no setter; read it with `get_config`. `node app/demo.js lock` deletes the coin account's full-access keys. |
| Sybil resistance good enough for a demo, limits written plainly | Anti-farm weighting below; [Limits](#limits-stated-plainly); the testnet run includes a farm we deliberately *don't* catch. |
| Page with the current streak, today's progress and a log of every mint and burn | `web/index.html`: streak with best-ever "high score", today's quorum bar, daily log, and the **on-chain supply ledger** (`get_ledger`), which the page checks against total supply. |
| Simulate 60 days under a few scenarios, then pick parameters | `sim/`: 4 scenarios × naive vs Grassroots, 60 days, supply-over-time chart, parameters chosen from the results. |

## The problem

Memecoins started as community: Dogecoin was a tipping currency, and its holders crowdfunded the Jamaican bobsled team's trip to the 2014 Olympics. Today most launches are built to be dumped on the next buyer:
- insiders and snipers buy in the first block;
- "drop ur address for 100 [coin]" engagement farming;
- Telegram pump groups;
- rugs.

Launchpads (pump.fun on Solana, meme.cooking on NEAR) make *launching* easy, but the coin they produce is a plain fixed-supply token with no rules after launch.

## What Grassroots is

A NEP-141 token contract with the community rules **inside the token**. After deployment the account's full-access keys are deleted, so nobody, including us, can change them.

| Track phrase | Grassroots rule |
|---|---|
| Supply that reacts to the community | 60% of max supply starts **unminted**. A tranche mints only when **independent** holders (wallet clusters counted once, held ≥ N days) cross a milestone. If the community shrinks below the last milestone for a while, part of the reserve is **cancelled forever**. |
| Treasury with rules | A community pool with **no owner**. It can only pay the daily ritual, pay capped creator challenges, burn on a missed day, or receive season bonuses. |
| On-chain ritual | **Daily check-in.** If unique independent check-ins reach the quorum, a slice of the pool goes to that day's check-ins; if not, a slice burns and the community streak resets. Plus a voluntary **weekly bonfire** (burn) with an on-chain title. |
| Challenges | **Seasons:** grow independent holders by a target or the season bonus is cancelled. **Creator challenges:** any game or activity can plug in, proven by an ed25519 signature from the creator's server or by a NEAR verifier contract. |

**Anti-farm, applied to every payout:**

`weight = independence share × holding-age ramp × streak bonus (≤1.5×) × stake (linear up to a cap)`

- Capped linear stake means **splitting a stack across many wallets earns nothing extra**.
- A detected cluster of k wallets shares one weight.
- New holders ramp in over `age_full_days`, so pump-and-leave buyers earn little.
- The creator's allocation (20% of each tranche) unlocks **only** through real holder growth.

**No drain while launching:** the day clock starts only when the oracle first reports `quorum_min` independent holders. Before that there are no days, burns or seasons, so a coin that is still being distributed can't lose its pool or reserve (`idle_coin_is_not_drained_before_the_ritual_starts`).

**Never a honeypot:** there is no transfer tax and no sell lock. Plain NEP-141 transfers always work, and pools on Rhea/Ref work normally.

## How NEAR is used

| Need | NEAR feature |
|---|---|
| The rules live in the coin | Each NEP-141 token is its own contract (on Solana, standard SPL tokens share one program) |
| Nobody can change them | Delete the account's full-access keys after deploy (`node app/demo.js lock`) |
| Verify creator challenge proofs on-chain | `env::ed25519_verify` |
| Plug-in verifier contracts | Async cross-contract call + callback (`claim_verified` → `on_verified`) |
| Auditable history | NEP-297 `ft_mint` / `ft_burn` / `ft_transfer` events for every reward, tranche and burn |
| Readable identity, clustering evidence | Named accounts and sub-accounts |
| Oracle reads holders without an indexer | `get_holders` view over any public RPC |

## Repository

| Path | What |
|---|---|
| `contracts/grassroots/` | Rust contract (`src/lib.rs`) and 25 unit tests (`src/tests.rs`) |
| `sim/` | 60-day community simulation used to choose parameters, with results and charts |
| `app/oracle.js` | Organic Score oracle v1: reads holders, clusters wallets, signs and submits attestations, writes evidence JSON |
| `app/demo.js` | Testnet demo: deploys a coin and runs organic users, a 30-wallet sybil farm and creator-side insiders through it |
| `web/index.html` | Coin page (static, reads the contract live over RPC) |
| `research/` | Hackathon landscape research (336 projects) and design notes |

## Try it

**Unit tests** (Rust ≥ 1.80):
```sh
cd contracts/grassroots && cargo test
```
Do X → see Y:
- `splitting_a_stack_earns_nothing_extra`: 3 wallets holding 1,000 each weigh the same as 1 wallet holding 3,000.
- `detected_cluster_shares_one_weight`: a flagged 3-wallet farm weighs like one account and counts once toward quorum.
- `creator_gets_nothing_without_real_growth`: `claim_creator` panics before any milestone.
- `challenges_cannot_drain_the_pool`: a creator challenge with reward = max supply pays at most 1% of the pool per day.
- `flagged_insiders_can_always_sell`: a flagged wallet can transfer its whole balance (no honeypot).
- `rejects_forged_attestation` / `rejects_replayed_attestation`: the oracle can't be impersonated or replayed.

**Simulation:**
```sh
uv venv .venv && uv pip install --python .venv/bin/python matplotlib
.venv/bin/python sim/grassroots_sim.py
```
→ `sim/results/summary.csv`: a 300-account sybil farm captures 54% of rewards on a naive streak coin vs 14% on Grassroots, and earns about the same per token held as an honest holder.

**Testnet demo** (Node ≥ 18, `cargo-near`):
```sh
cd contracts/grassroots && cargo near build non-reproducible-wasm
cd ../../app && npm install
node demo.js setup <your-name>.testnet   # creates accounts via the testnet faucet, deploys, distributes
node demo.js run 6                       # 6 demo "days" (4 min each): oracle attests, actors check in
node demo.js challenge                   # creator plugs in a signed challenge; farm wallets get capped
node demo.js report                      # rewards by group: organic vs farm (caught / hidden) vs insiders
```
Open `web/index.html?coin=<coin id printed by setup>` to watch the streak, tranches, burns and flags live.

## Testnet results

Live coin: [`gdog-1huvn.testnet`](https://testnet.nearblocks.io/address/gdog-1huvn.testnet). Its full-access keys are deleted, so the contract can no longer change. The run: 7 demo days of 4 minutes each, then a creator challenge. Raw output is in `app/data/demo-report.json`.

| Group | Wallets | Tokens held | Share of rewards |
|---|---|---|---|
| Organic holders (show up ~80% of days) | 12 | ~702k | **70.0%** |
| Wallet farm, caught (sub-accounts of one farmer) | 24 | 480k | **1.3%** |
| Wallet farm, *not* caught (implicit accounts) | 6 | 120k | 28.6% |
| Creator-side insiders | 4 | 1.6M | **0.09%** |

- Streak 6 with no missed day, and 3 growth tranches minted as independent holders crossed 5, 10 and 15.
- The supply ledger has 21 entries and reconciles with total supply.
- The creator challenge ("Daily meme raid", 2,000 per completion) paid 2,000 to organic holders, 83 to caught farm wallets and 2 to insiders. A second claim the same day was refused.
- **The honest gap:** the 6 uncaught farm wallets check in every single day, so per token held they earn about 2× what an organic holder does. Splitting gives them nothing extra, but perfect attendance does. Better clustering (funding-source analysis) is the fix; see Limits.

## Limits (stated plainly)

- **Oracle trust.** Independent-holder counts and cluster flags come from a signed off-chain oracle. The contract trusts its key, but each flag's evidence is published (`app/data/score-*.json`), and a forged or replayed attestation is rejected.
- **Clustering v1 is simple.** It catches wallets that share a parent account and the creator/launch side. It does **not** catch unrelated-looking accounts funded through exchanges; the demo includes 6 such "hidden" farm wallets to show the gap. Capped linear stake weighting still means those wallets earn no more than the same coins held in one account.
- **Pump groups made of real people** can earn while they hold. The 14-day ramp and the 14-day milestone hold keep their share small and stop them from triggering supply.
- **Demo parameters** (4-minute days, small milestones) are for showing the full life of a coin in minutes. Production parameters are in `sim/README.md`.
- **The pool pays rewards instead of minting per check-in** (the brief's example mints). A per-check-in mint is exactly what a wallet farm targets, so Grassroots funds the pool only from growth tranches and season bonuses, which are themselves gated on independent holders.
- **Not yet integrated with meme.cooking.** Next step: a meme.cooking-launched coin could graduate into Grassroots rules by migrating into a wrapper contract.
- Testnet only. Not audited.

## Built with

- `near-sdk` 5 and `near-contract-standards` (NEP-141, NEP-145)
- `near-api-js` 5
- `matplotlib` (simulation charts only)
- AI coding assistance (Claude Code)

No pre-hackathon code.

## License

MIT
