# Grassroots 60-day simulation

`grassroots_sim.py` models 60 days of a coin's life under four scenarios. It compares a **naive streak coin** (every check-in weighs the same, every account counts as a holder) with **Grassroots** rules. Its purpose is to choose the contract parameters before writing the contract.

```sh
uv venv .venv && uv pip install --python .venv/bin/python matplotlib   # charts only
.venv/bin/python sim/grassroots_sim.py                                  # writes sim/results/
```

## Scenarios
| Scenario | Actors |
|---|---|
| organic | 900 real people arriving on an S-curve, each with their own check-in habit, small daily churn |
| insider_pump | 600 organic + **30 deployer-linked insider wallets** (front-run launch, dump day 12) + **250-person Telegram pump group** (arrive days 4–8, leave days 14–18) |
| sybil_farm | 600 organic + **one farmer running 300 accounts** (80% caught by clustering, 20% look independent), checking in every day |
| dead | 120 people arrive in the first 20 days, then nobody |

## Results (seed 7)

**Share of all rewards captured by bad actors**

| Actor | Naive coin | Grassroots |
|---|---|---|
| Sybil farm (300 accounts) | **54.3%** | **13.8%** |
| Insiders (30 wallets) | 1.3% | 0.7% |
| Telegram pump group (250 humans) | 4.3% | 4.8% |

**Reward per token held (does splitting into many accounts pay?)**

| | Naive | Grassroots |
|---|---|---|
| Sybil farm | **0.78** | 0.17 |
| Organic holders, same scenario | 0.10 | 0.15 |

Under Grassroots, the farm earns about the same per token as an honest holder. **Running 300 accounts earns nothing extra over holding the same coins in one account.** In the naive coin, the farm earns 8× per token.

**Supply follows the real community** (`results/supply_vs_community.png`): supply unlocks in steps as independent holders who've held ≥14 days pass milestones. The dead community stalls at 54% of max supply with its season bonus cancelled; the organic one reaches 74% by day 60.

## What the simulation changed

1. **Flat per-account rewards are farmable.** Version 1 weighted every check-in equally, and the 20% of sybils that clustering missed still captured 32% of rewards. **Fix: reward weight grows with balance up to a cap** (0.05% of max supply). Below the cap, splitting a stack across accounts earns nothing extra, and clustering handles whales who split above it.
2. **Bots never miss a day.** The streak multiplier was 2× at 30 days, which mostly rewarded scripts. **Fix: cap the streak bonus at 1.5×.**
3. **A 7-day hold let a pump group trigger supply.** The Telegram group (real, separate humans, so clustering can't catch them) held 7+ days and triggered two growth tranches before leaving. **Fix: milestones count only holders who've held ≥14 days.** The insider+pump supply curve now tracks the organic one.

## Chosen parameters (input to the contract)

| Parameter | Value |
|---|---|
| Max supply | 1,000,000,000 |
| At launch | 35% sold / LP, 5% community-pool seed, 60% unminted growth reserve |
| Milestones (independent holders ≥14 days) | 50, 100, 200, 400, 800, 1600, 3200 |
| Tranche sizes (% of max supply) | 5, 7, 9, 11, 12, 13, 13 |
| Tranche split | 80% community pool / 20% creator earn-out |
| Contraction | below last milestone 7 days in a row → cancel 10% of remaining reserve |
| Daily quorum | max(10, 50% of independent holders) |
| Daily payout / miss burn | 0.5% of pool / 0.25% of pool |
| Reward weight | independence share × min(1, days held/14) × (1 + 0.5·min(streak,30)/30) × min(balance, cap)/cap |
| Balance cap | 0.05% of max supply |
| Season | 30 days, goal +50% independent holders, bonus 2% of max supply (cancelled if missed) |

## Honest limits
- **Pump groups made of real humans aren't stopped from earning rewards** while they hold, only from triggering supply. They are real participants for those days, and the 14-day age ramp keeps their share small (4.8%).
- **The daily quorum was never missed** in any scenario (best streak 60/60). With these check-in habits the 50% quorum is comfortable. The "miss → burn" path is tested in contract tests, not exercised by this model. Raise it if the ritual should carry more tension.
- **Detection rates are assumptions** (80% sybil, 90% insider). Real rates depend on the Organic Score heuristics.
- This is one seed and a stylized model, not a forecast.
