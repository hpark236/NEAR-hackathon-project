# Judging INSYDR

Every claim, how to check it, what you should see, and the test that covers it. Nothing below needs a wallet or real funds.

## Fastest path

| Time | Do this | Expect |
|---|---|---|
| 2 min | Open **https://hpark236.github.io/NEAR-hackathon-project/?run=all** | The "Check every claim yourself" panel ticks 7 of 7 to PASS while the terminal shows each live call. Results are also on `window.INSYDR_CHECKS`: `locked`, `accounting`, `split`, `farms`, `ratio`, `checkin` and `attacks`, each `true` when it passed. It creates one throwaway testnet guest and signs about 10 testnet transactions. |
| 2 min | `cd contracts/insydr && cargo test` | `test result: ok. 44 passed` (first build about 2 minutes) |
| 4 min | `cd app && npm ci && npm test && npm run e2e` | `# pass 13`, then ten simulated days, then `24/24 checks passed` and exit code 0 |
| 1 s | `python3 sim/insydr_sim.py && python3 sim/strength_test.py && git status --short sim docs` | Prints the tables and leaves the tree clean: the committed results regenerate byte for byte (checked on Python 3.9, 3.10, 3.12 and 3.14; CI runs 3.10) |

Headless check of the live page, for agents:

```bash
# Node 18+ with Playwright or Puppeteer: open the page, wait for the checklist, read the results
#   page.goto("https://hpark236.github.io/NEAR-hackathon-project/?run=all")
#   await page.waitForFunction(() => ["locked","accounting","split","farms","ratio","checkin","attacks"].every((k) => k in window.INSYDR_CHECKS), { timeout: 240000 })
#   console.log(await page.evaluate(() => window.INSYDR_CHECKS))
```

Without a browser, the two chain facts that matter:

```bash
RPC=https://test.rpc.fastnear.com
# 0 access keys: prints "keys":[]
curl -s $RPC -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_access_key_list","finality":"final","account_id":"idog-bqdss4.testnet"}}'
# deployed code hash A2K4Vj76jmrqGBG5FRHVsnB6CRgauYAZYWBtFkciXNNZ
curl -s $RPC -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_account","finality":"final","account_id":"idog-bqdss4.testnet"}}'
# = base58(sha256(contracts/insydr/res/insydr.wasm)); hex 860f2f17...c9d8b6
python3 -c "import hashlib;b=hashlib.sha256(open('contracts/insydr/res/insydr.wasm','rb').read()).digest();A='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';n=int.from_bytes(b,'big');s=''
while n: n,r=divmod(n,58); s=A[r]+s
print(s)"
```

## Claim by claim

| Claim | Live check (page terminal) | Offline check |
|---|---|---|
| Nobody can change the coin: 0 access keys, no admin methods, the deployed code is the committed wasm | `verify`: 9 PASS lines. It reads the key list, fetches `insydr.wasm` from this repo, hashes it in the browser and compares it with the deployed code hash, and lists the contract's exported methods from its own wasm | e2e: `after the keys are deleted nobody can redeploy or reconfigure the coin` |
| Every coin is accounted for | `verify`: ledger minted minus burned equals supply; supply + burned + reserve + cancelled + creator's unclaimed = max supply | `fuzz_accounting_invariants_hold` (6,000 random operations), `ledger_records_every_mint_burn_and_cancel` |
| Splitting a stack across wallets earns nothing | `split 3000000 30`: same weight for 1 wallet and 30 | `splitting_a_stack_earns_nothing_extra`, `rotating_one_stack_through_wallets_does_not_multiply_it`, `challenge_rewards_scale_with_stake_so_splitting_is_neutral` |
| Farms are flagged, and the evidence is on-chain | `farms`: each cluster with each wallet's recorded coin source; `whois <account>` for one wallet | `majority_source_shows_where_a_wallets_coins_came_from`, `detected_cluster_shares_one_weight`; `npm test` (13 clustering tests) |
| A farm earns a fraction of an independent holder | The "Who gets paid" panel, from every holder's on-chain rewards (live: a farm wallet earns about 6% and a creator-side wallet about 0.1% of an independent holder, per coin) | e2e: `flagged farm wallets earn under 10%`, `creator-side wallets earn under 1%`; `docs/STRENGTH-TEST.md` |
| Fresh or dust wallets cannot pump the day | `checkin` as a new guest: weight about 0 (coin-age starts at zero) | `new_holders_ramp_in`, `dust_aged_wallet_gains_nothing_when_loaded`, `dust_wallets_cannot_move_the_daily_target` |
| Admin and oracle attacks fail | `attack all`: 7 real failed transactions from your guest (admin mint, set_config, upgrade, forged attestation, challenge, creator claim, double check-in) | `rejects_forged_attestation`, `rejects_replayed_attestation`, `one_oracle_alone_cannot_attest_when_two_are_required`, `oracle_cannot_report_more_holders_than_exist`, `a_compromised_oracle_cannot_mint_the_reserve_at_once`, `only_creator_adds_challenges`, `a_stranger_cannot_trigger_the_creators_claim` |
| A missed day burns and resets the streak | `day 1` on the live coin shows a real MISS and its burn; `ledger` lists it | `missed_quorum_burns_pool_and_resets_streak`, `empty_days_burn_too`; e2e day 5 |
| The target follows the community, not a constant | `status`: today's target is a share of the coin's own trailing turnout | `the_daily_target_follows_the_communitys_own_turnout` |
| The creator only earns from real growth, and loses it if growth was fake | `verify`: claimed <= vested <= granted; the Treasury panel | `creator_gets_nothing_without_real_growth`, `creator_cut_vests_linearly`, `contraction_claws_back_the_unvested_creator_cut` |
| Selling is never blocked | Any wallet can `ft_transfer` | `flagged_insiders_can_always_sell`; e2e: `a flagged insider can still sell everything` |
| 60-day simulation picked the parameters | `sim/README.md`, `sim/results/summary.csv` | `python3 sim/insydr_sim.py` regenerates them |

## What is simulated, and the known limits

- **The live community is scripted.** 38 testnet accounts (12 "real holders", three kinds of farm, the creator's wallets) check in through `node demo.js keep`, which a GitHub Actions job runs hourly. The visitor count on the page counts only accounts the demo tap served from the page, which includes the guests created while testing this page.
- **The oracle is a 2-of-3 committee**, bounded on-chain (one tranche per day, never more holders than exist, it cannot touch balances or rules or stop the daily ritual). Attestations do not name the coin, so a second coin that reused the same oracle keys could replay them; each coin needs its own keys.
- **The demo tap keeps an access key** (`tap.creator-bqddng.testnet`). It is a faucet for the wallet-free trial and holds no power over the coin.
- **Wallet connection** uses NEAR Connect (Meteor, Intear, HOT, Ledger, Nightly and others) on testnet. The picker and the signing path are wired; it was tested up to the wallet picker, not with a funded wallet. Guest mode is the path that was tested end to end.
- Not audited. Testnet only.
