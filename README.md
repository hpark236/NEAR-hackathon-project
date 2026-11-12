# INSYDR

> **Everyone's an insider. Except the insiders.**

**INSYDR is a NEP-141 memecoin contract that pays the people who show up and nobody else.** Holders check in once a day. If the community's turnout holds, the pool pays that day's check-ins; if it drops, part of the pool burns and the streak resets. New supply mints only when the number of *independent* holders grows. Wallet farms, split stacks and the creator's own wallets earn close to nothing, and after launch nobody can change a rule.

Built at NEAR HACKATHON (November 2026). Tracks: **Memecoins With Mechanics** (primary), Social Money, AI Security & Defense.

| | |
|---|---|
| **Live coin (testnet)** | [`idog-bqdss4.testnet`](https://testnet.nearblocks.io/address/idog-bqdss4.testnet): 12-hour days, 2-of-3 oracle, **0 access keys since deployment** |
| **Coin page** | **https://hpark236.github.io/NEAR-hackathon-project/** (static; reads the contract live; has a test terminal). Source: `web/index.html` |
| **Run everything without a wallet** | `npm run e2e`: the real wasm on a local NEAR sandbox, 24 checks, about 2 minutes |
| **Tests** | 44 contract unit tests (one is a 6,000-operation accounting fuzz), 13 oracle tests, 24 end-to-end checks |
| **Evidence** | [60-day simulation](sim/README.md) and a [strength test](docs/STRENGTH-TEST.md): what broke when we attacked our own design, what we changed, what is still weak |
| **Tested binary** | [`contracts/insydr/res/insydr.wasm`](contracts/insydr/res/insydr.wasm), the exact code deployed (hash check below) |

## The problem

Memecoins began as communities. Today most launches are built to be dumped on the next buyer: insiders buy in the first block, farmers run fifty wallets to collect every giveaway, and the creator can change the rules or leave. Launchpads make *launching* easy, but the coin they produce has no rules after launch.

The track's example prompt is a daily check-in streak coin. Built naively, that coin pays per wallet, so it pays whoever makes the most wallets. In our simulation one farmer holding 13% of the coins takes **64% of all rewards with 300 wallets and 95% with 3,000**. INSYDR is that coin with the farm, the insiders and the admin key taken out.

## Try it

### 1. Without a wallet (everything an automated check can run)

```sh
git clone https://github.com/hpark236/NEAR-hackathon-project && cd NEAR-hackathon-project

# the rules: 44 unit tests, about 2 minutes (Rust stable)
cd contracts/insydr && cargo test && cd ../..

# the whole product on a local NEAR chain: about 2 minutes (Node 22)
cd app && npm install && npm test && npm run e2e && cd ..

# the evidence: 60-day simulation and seven attack experiments (Python 3.10+, no packages needed)
python3 sim/insydr_sim.py && python3 sim/strength_test.py
```

`npm run e2e` needs no Rust: it uses the committed wasm unless you have built your own (`cargo near build non-reproducible-wasm`). On macOS the sandbox binary needs `xz` (`brew install xz`); Linux needs nothing.

**Do X, see Y:**

| Run | Expect |
|---|---|
| `cargo test` | `44 passed`. Read the test names: each one is a claim in this README (`splitting_a_stack_earns_nothing_extra`, `dust_aged_wallet_gains_nothing_when_loaded`, `a_compromised_oracle_cannot_mint_the_reserve_at_once`, …). |
| `npm test` | `13` oracle clustering tests pass. |
| `npm run e2e` | Ten simulated days print one line each, then a results table, then `24/24 checks passed`. Day 5 is a deliberate `MISS` (most real holders stay away): the pool burns and the streak resets. Exit code 0. A report is written to `app/data/e2e-report.json`. |
| `python3 sim/strength_test.py` | Seven tables (E1 to E7) and `docs/STRENGTH-TEST.md` regenerated with the same numbers. |

What the end-to-end run shows (one run; yours will be within a few percent):

| Group | Wallets | Flagged | Rewards per 1,000 coins held | Share of all rewards |
|---|---|---|---|---|
| Real holders (show up about 85% of days) | 8 | 0 | **672** | 73.7% |
| Farm, sub-accounts of one account | 6 | 6 | **64** | 3.1% |
| Farm, random-looking wallets fed from one stack | 4 | 4 | **64** | 2.1% |
| Farm, each wallet bought separately (not detected) | 3 | 0 | 870 | 21.0% |
| Creator's named wallets | 2 | 2 | **0.7** | 0.07% |
| Creator's unnamed wallets (fed by the creator) | 2 | 2 | **0.7** | 0.07% |

The undetected farm is the honest limit: it earns 1.3 times a real holder per coin, which is its attendance advantage (it never misses a day) and nothing more. It gets no bonus for being three wallets.

### 2. The live contract

```sh
curl -s https://test.rpc.fastnear.com -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"call_function","finality":"final","account_id":"idog-bqdss4.testnet","method_name":"get_state","args_base64":"e30="}}'
```
The result is a byte array of JSON: streak, turnout target, supply, reserve, burned, cancelled.

Check that nobody can change it:
```sh
# no access keys: prints "keys": []
curl -s https://test.rpc.fastnear.com -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_access_key_list","finality":"final","account_id":"idog-bqdss4.testnet"}}'
# the deployed code is the committed binary: code_hash A2K4Vj76jmrqGBG5FRHVsnB6CRgauYAZYWBtFkciXNNZ
#   = base58 of  shasum -a 256 contracts/insydr/res/insydr.wasm  (860f2f17…c9d8b6)
curl -s https://test.rpc.fastnear.com -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_account","finality":"final","account_id":"idog-bqdss4.testnet"}}'
```

### 3. The page and its test terminal

Open **https://hpark236.github.io/NEAR-hackathon-project/** (or serve it yourself: `cd web && python3 -m http.server 8080`, then `http://localhost:8080/?coin=idog-bqdss4.testnet`).

The page reads everything from the contract over public RPC. The terminal in the middle of it runs live calls:

| Type | See |
|---|---|
| `verify` | Eight on-chain checks with PASS or FAIL: 0 access keys, the ledger reconciles with supply, every coin is accounted for, 2-of-3 oracle, and a list of the contract's public methods read from its own wasm, showing there is no mint, setter, pause or upgrade. |
| `farms` | Every flagged cluster with the evidence the contract itself holds (which account most of each wallet's coins came from). |
| `split 30000000 30` | Whether splitting that stack across 30 wallets earns more (it gets around the whale cap, and what detection does about it). `split 3000000 30`: same weight, splitting earns nothing. |
| `join` | Creates a throwaway testnet account in your browser and gets 50,000 coins from the demo tap. No wallet. |
| `checkin` | Signs a real `check_in`. Fresh coins weigh almost nothing, which is the rule against pump-and-leave. |
| `attack all` | Seven real transactions that try to break the coin as your guest account: call a mint, change the config, upgrade the code, forge an oracle attestation, register a challenge, claim the creator's earn-out, check in twice. Each is refused by the contract and the refusal message is printed. |

## For judges

Previous NEAR hackathons scored technical implementation, design, impact, originality and sustainability. Where each is answered, with the weak spot stated.

| Criterion | Where | The honest caveat |
|---|---|---|
| **Technical implementation** | One Rust contract holds every rule ([`lib.rs`](contracts/insydr/src/lib.rs)). 44 unit tests including a randomized test that performs 6,000 operations and checks the accounting after each. 24 end-to-end checks on the real wasm. The live contract has 0 access keys and its code hash matches the committed binary. | Not audited. Testnet only. Once the keys are deleted a bug cannot be patched. |
| **Design** | A coin page whose every number is read from the chain, with a terminal that lets a visitor verify the claims and attack the contract without a wallet. | Guest mode depends on the public testnet faucet and free RPC endpoints, which rate-limit. |
| **Impact** | A streak coin that cannot be farmed by wallet count: in the simulation the farm's take falls from 64% to 21% (0.3% when fully detected), and it stops scaling with the number of wallets. The contract is a template any creator can deploy with their own parameters. | A well-funded farmer who buys separately for every wallet and waits is treated as that many holders. No design without identity stops that. |
| **Originality** | Coin-age instead of wallet-age; a daily target that retargets to the community's own turnout like a difficulty; an on-chain record of where each wallet's coins came from, which makes farm clustering checkable by anyone; a creator cut that vests and is clawed back if the growth was fake. | Burns, streaks and holder milestones are familiar tools. What is specific here is how they are combined so that none of them can be gamed by splitting wallets. |
| **Sustainability** | No admin, no server needed for the daily ritual, anyone can call `settle`. The oracle is bounded and its evidence is on-chain. | The pool is funded by growth. When growth stops, rewards shrink by design. The oracle is still a committee of signers, not a proof. Contract storage is paid from the contract's NEAR balance, which someone has to top up. |

**Against the track's example prompt** ([`research/official/11-memecoins.md`](research/official/11-memecoins.md)):

| The prompt asks for | Status |
|---|---|
| An NEP-141 token with the mechanic in the contract, not in a backend somebody could switch off | Yes. Check-in, turnout target, payout, burn, streak, seasons, challenges and the ledger are in the contract. The daily ritual uses no off-chain input at all. The oracle only reports the independent-holder count (which gates new supply) and cluster flags. If it stops, the ritual continues and supply simply stops growing. |
| All parameters set at launch and readable by anyone. No admin key that can change them later | Yes. `Config` is an argument of `new` and has no setter (`get_config` to read it). The live contract has 0 access keys. The one thing the creator can do after launch is register challenges, inside caps fixed at launch. |
| Sybil resistance good enough for a demo, with its limits written plainly | Yes, and it is the point of the project. [Limits](#limits) and [docs/STRENGTH-TEST.md](docs/STRENGTH-TEST.md). |
| A page with the current streak, today's progress and a log of every mint and burn | Yes: streak and best streak, a turnout meter against today's target, a streak calendar, and the on-chain ledger (`get_ledger`) of every mint, burn, cancellation and payout, which the page reconciles against total supply. |
| Simulate 60 days under a few scenarios, then pick the parameters | Yes: [`sim/`](sim/README.md), four communities, naive versus INSYDR. The simulation changed the design twice (the daily target and the streak rule). |
| "Every holder who checked in gets a small mint" | Different on purpose. A per-check-in mint is what a farm targets. INSYDR mints only on independent-holder growth and pays check-ins from the pool. |

## How it works

| Rule | In the contract |
|---|---|
| **Supply follows the community** | 60% of max supply starts unminted. A tranche mints when independent holders pass a milestone, at most one per `tranche_cooldown_days`. 80% goes to the community pool. If holders stay below the last milestone, 10% of the reserve is cancelled for good. |
| **Daily ritual** | `check_in` once per day. Turnout is the sum of the weights that checked in. Hit the target and `daily_payout_bps` of the pool is paid to that day's check-ins by weight. Miss it and `miss_burn_bps` burns and the community streak resets. |
| **The target sets itself** | The target is `quorum_bps` (75%) of the community's trailing-average turnout. A steady community keeps its streak at any attendance level; a real drop is punished until the target has come down to the community that is left. |
| **Weight** | `independence × coin-age × stake × streak`. Stake is linear up to a cap, so splitting a stack earns nothing. Coin-age follows the coins, so a pre-aged empty wallet gains nothing when loaded. A flagged cluster of k wallets shares one weight. The streak bonus is at most +50% and a missed day costs one day of streak instead of resetting it. |
| **Who counts as a holder** | Only an account with `holder_min_balance` coins aged `holder_min_days`, with wallet clusters counted once and the creator's side never. |
| **Creator** | Gets 20% of each tranche, so nothing without real growth. It vests over `creator_vest_days` and the unvested part is cancelled if the community contracts. |
| **Creator challenges** | Any game or activity can plug in: the creator's server signs a completion (ed25519, verified on-chain) or a NEAR contract answers `verify`. Payouts use the same weight, a daily cap for all challenges together and a per-account share of that cap. [`app/challenge-server.js`](app/challenge-server.js) is a 60-line example. |
| **Seasons and bonfire** | Every `season_days`: grow independent holders by the goal and a bonus mints to the pool, otherwise it is cancelled. Anyone can burn their own coins into the weekly bonfire; the largest offering holds the week's title on-chain. |
| **Never a honeypot** | No transfer tax, no sell lock. A flagged wallet can always transfer and sell. Flags only reduce rewards. |
| **Not drained while launching** | The day clock starts when the oracle first reports `quorum_min` independent holders. |

## How NEAR is used

| Need | NEAR feature |
|---|---|
| The rules live in the coin | Every NEP-141 token is its own contract, so the token itself can carry the ritual, the pool and the ledger. |
| Nobody can change them | An account's access keys can be deleted. With none left the code and state can only change by the contract's own rules (`node demo.js lock`). |
| Verify oracle and challenge signatures on-chain | `env::ed25519_verify`, for 2-of-3 oracle attestations and signed challenge completions. |
| Plug-in challenge verifiers | Asynchronous cross-contract call with a callback (`claim_verified` → `on_verified`). |
| Farm evidence without an indexer | The contract keeps holders, coin-age and each wallet's majority token source in its own state; `get_holders` returns all of it over any public RPC. |
| Readable identity | Named accounts and sub-accounts: `f1.farm.near` and `f2.farm.near` visibly share an owner. |
| A log of every mint and burn without an indexer | The contract keeps its own ledger (`get_ledger`), alongside the standard NEP-297 events. |
| A wallet-free trial | Named testnet accounts created from a browser, and a 60-line tap contract ([`contracts/tap`](contracts/tap/src/lib.rs)) that calls the token. |
| Daily check-ins that cost almost nothing | A check-in is one function call, well under a cent of gas. |

## Strength test

We attacked the design before asking anyone to trust it. Nine things broke and were fixed in the contract; each fix is pinned by a named test. The short version:

| What broke | What the contract does now |
|---|---|
| A wallet holding 1 coin for weeks, then loaded, looked like an old holder | Coin-age: an inflow dilutes age in proportion |
| "50% of holders check in daily" was missed on every one of 60 simulated days | The target retargets to the community's own turnout |
| One oracle key could mint the whole reserve in one transaction | 2-of-3 signatures, one tranche per cooldown, count bounded by on-chain holders, creator cut vests with clawback |
| Challenge rewards ignored stake, so 100 dust wallets earned 100 rewards | Same weight as the ritual, plus a per-account cap |
| A streak that resets on a miss paid scripts up to 1.5 times more | A miss costs one day of streak |
| Farm flags could only be checked against a file we publish | The contract records each wallet's majority token source |

The full list, seven experiments with numbers (bots against people, realistic attendance, a sudden exodus, faking a milestone, a stolen oracle, a farm split across 1 to 3,000 wallets, a creator draining challenges) and nine things that are still weak: **[docs/STRENGTH-TEST.md](docs/STRENGTH-TEST.md)**.

## What is real and what is not

| Claim | Status |
|---|---|
| The contract is deployed on NEAR testnet and locked | **Real.** `idog-bqdss4.testnet`, 0 access keys, code hash matches the committed wasm. Commands above. |
| The whole scenario runs against the real wasm | **Real**, on a local NEAR sandbox (`npm run e2e`). The sandbox fast-forwards its clock, so ten days take two minutes. |
| The live coin has a community | **Simulated community, real transactions.** 38 scripted testnet accounts (12 "real holders", three kinds of farm, the creator's wallets) check in through a keeper script (`node demo.js keep`), about once per 12-hour coin day. Misses happen when the machine running the keeper is asleep; they are real burns. |
| The oracle is 2-of-3 | **Enforced by the contract.** In this demo one process holds all three keys. In production each signer would run `app/oracle.js` separately; the scoring is a pure function of on-chain state, so they compute the same payload. |
| A visitor can check in from the page without a wallet | **Real on testnet**, exercised in Chrome: guest account from the public faucet, coins from the tap contract, a signed `check_in`, and the attack transactions. |
| The page works with a NEAR wallet (MyNearWallet, Meteor) | **Not built.** The page signs only with its own throwaway guest key. |
| Farm and insider results | **Measured** in the sandbox run and on the live coin (the page's "Who gets paid" chart computes them from the contract). The 60-day figures are **simulated**. |
| meme.cooking integration | **Not built.** A coin launched there has a fixed supply in a factory contract; INSYDR's rules need its own contract. Bridging the two (a wrapper a meme.cooking coin migrates into) is future work. |
| Mainnet | **Not deployed.** Unaudited. |

## Live deployment

| | |
|---|---|
| Coin | `idog-bqdss4.testnet` (INSYDR Dog, $IDOG, 18 decimals, max supply 1,000,000,000) |
| Creator | `creator-bqddng.testnet` |
| Launch account | `launch.creator-bqddng.testnet` (holds the unsold launch allocation; flagged creator side) |
| Demo tap | `tap.creator-bqddng.testnet` (50,000 $IDOG once per account, below `holder_min_balance`, so tap accounts never count toward supply) |
| Parameters | [`demoConfig` in `app/scenario.js`](app/scenario.js): 12-hour days, milestones at 5 / 10 / 15 / 40 independent holders, 0.3% of the pool per hit day, 1% burned per miss. These are demo-scale so that a small scripted community exercises every rule within the judging window. Production values are in [`sim/README.md`](sim/README.md). |

## Repository

| Path | What |
|---|---|
| `contracts/insydr/` | The coin: `src/lib.rs`, 44 tests in `src/tests.rs`, tested binary in `res/` |
| `contracts/tap/` | Demo-only faucet contract for the wallet-free trial |
| `app/oracle.js` | Organic Score oracle: reads holders, clusters wallets (pure function), signs and submits attestations, writes evidence JSON |
| `app/scenario.js` | The demo scenario, shared by the sandbox run and testnet |
| `app/e2e.js` | Sandbox end-to-end run with 24 checks |
| `app/demo.js` | Testnet runner and the live coin's keeper |
| `app/challenge-server.js` | Example creator challenge server |
| `web/index.html` | Coin page and test terminal (one static file, no build step) |
| `sim/` | 60-day simulation and strength-test experiments, with results |
| `docs/STRENGTH-TEST.md` | What broke, what changed, what is still weak |
| `research/` | Hackathon landscape research and design notes |

## Limits

- **A patient, well-funded farmer is a holder.** Wallets that each buy separately, are funded from unrelated sources and hold for weeks are not detected. They earn what the same coins would earn in honest hands and no more, and they get around the whale cap.
- **Checking in is scriptable.** INSYDR does not prove a human is present. It makes automation worth what attendance is worth, and wallet count worth nothing.
- **The oracle is trusted for two things**: when supply tranches mint and which wallets are flagged. It is bounded and its evidence is on-chain, but the signers are a committee. A wrong flag lowers a wallet's rewards; it can never freeze or take coins.
- **Rewards are redistribution, not yield.** People who show up are paid by diluting people who do not. When growth stops the pool decays and rewards shrink.
- **A locked contract cannot be patched.**
- **Demo parameters are demo-scale**, and the live community is scripted.
- Testnet only. Not audited.

## Built with

- `near-sdk` 5 and `near-contract-standards` (NEP-141, NEP-145); `near-api-js`; `near-sandbox` for the local chain; `matplotlib` for simulation charts only.
- The page is plain HTML, CSS and JavaScript. Fonts: Martian Mono and Instrument Sans. Its pointer spotlight, dot grid, text scramble and count-up effects are plain-JS takes on ideas from reactbits.dev.
- AI coding assistance: Claude Code wrote most of the code and documentation under the author's direction.
- No code predates the hackathon window.

## License

MIT
