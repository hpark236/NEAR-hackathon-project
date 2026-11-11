# INSYDR simulation and strength test

Two scripts, standard library only (matplotlib is optional and only draws the charts):

```sh
python3 sim/insydr_sim.py       # 60 days, four communities, naive coin vs INSYDR -> sim/results/
python3 sim/strength_test.py    # seven attack experiments -> sim/results/strength/ and docs/STRENGTH-TEST.md
```

Both are deterministic (fixed seeds). CI re-runs them and fails if the committed results differ.

## What the 60-day simulation models

One tick is one day. The rules mirror `contracts/insydr/src/lib.rs`: growth tranches with a cooldown, the creator's vesting cut and its clawback, the retargeting daily turnout target, weights (`independence x coin-age x capped linear stake x streak`), streak decay, seasons.

| Scenario | Who is in it |
|---|---|
| `organic` | 900 real people arriving on an S-curve. A quarter are regulars (check in 70-95% of days), the rest casual (5-40%). |
| `insider_pump` | 600 real people, 30 creator-side wallets holding 25x bags that dump on day 12, and a 250-person pump group that arrives days 4-8 and leaves days 14-18. |
| `sybil_farm` | 600 real people and one farmer running 300 wallets (13% of the coins) that never miss a day. 80% of the farm wallets are detected. |
| `dead` | 120 people arrive in the first 20 days, then drift away. |

The **naive** baseline is the track prompt's coin built the obvious way: every check-in weighs 1, the target is a fixed share of all accounts, a miss resets the streak.

## Results (`results/summary.csv`)

| | Naive coin | INSYDR |
|---|---|---|
| Farm's share of all rewards (300 wallets, 13% of coins) | 63.8% | 21.5% |
| Same farm, 3,000 wallets | 94.6% | 24.9% (0.0% if all detected) |
| Insiders' reward per coin held | 0.0063 | 0.0041 |
| Pump group leaving | no effect | 7 missed days, 0.15% of supply burned |
| Community that fades | supply ends at 45.4% | supply ends at 45.2%, 19% of supply cancelled, creator vested 0.23% (naive: 1.0%) |

`supply_vs_community.png` shows supply against independent holders for the four communities; `farmer_capture.png` shows who captures the rewards.

## Parameters chosen (production scale)

| Parameter | Value | Why |
|---|---|---|
| Launch / pool seed / reserve | 35% / 5% / 60% | Most supply has to be earned by growth |
| Milestones | 50, 100, 200, 400, 800, 1600, 3200 independent holders | Doubling steps |
| Tranches | 5, 7, 9, 11, 12, 13, 13% of max supply, 80% to the pool | Later growth is worth more |
| `tranche_cooldown_days` | 7 | A stolen oracle cannot mint faster than this (E5) |
| `creator_vest_days` | 90 (30 in the simulation) | Faking a milestone only pays if the fake holders stay (E4) |
| `holder_min_days` / `age_full_days` | 14 / 14 | Pump groups that leave within two weeks never unlock supply |
| Daily target | 75% of the trailing 7-day turnout, at least 10 independent check-ins | A fixed "50% of holders" target was missed on all 60 days at every attendance level (E2) |
| Payout / burn | 0.5% of the pool per hit, 0.25% per miss | A 75% exodus costs about 4% of the pool (E3) |
| Streak | up to +50% at 30 days, a miss costs 1 day | Reset-on-miss paid scripts up to 1.47x more than a person at 90% attendance (E1) |
| Whale cap | 0.05% of max supply per wallet | Below it splitting earns nothing; above it splitting is what cluster detection is for (E6) |
| Challenges | 0.1% of the pool per day in total, 5% of that per account | Worst case is a fifth of the ritual's payout (E7) |

The live testnet coin uses smaller, faster values (`demoConfig` in `app/scenario.js`) so that a scripted community of 38 wallets exercises every rule in days instead of months.
