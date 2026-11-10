### E1 Bots vs people: how much more a cron job earns per token than a person

| Person's attendance | Attendance alone | Streak resets on a miss | Decay, penalty 3 | Decay, penalty 2 | Decay, penalty 1 |
|---|---|---|---|---|---|
| 90% | 1.11x | 1.47x | 1.13x | 1.13x | 1.13x |
| 80% | 1.25x | 1.74x | 1.29x | 1.26x | 1.26x |
| 60% | 1.67x | 2.40x | 2.37x | 2.28x | 1.69x |
| 40% | 2.50x | 3.64x | 3.64x | 3.63x | 3.56x |

A wallet that never misses will always out-earn one that does. The question is by how much. With reset-on-miss, the streak bonus multiplies the gap. With decay (the contract's rule, `streak_miss_penalty`), the gap falls back to plain attendance for anyone who shows up most days. Chosen: penalty 1 (miss a day, lose a day). Below roughly 50% attendance no streak rule helps: the bonus is capped at 1.5x, so that is also the most a script can ever gain from it.

### E2 Realistic attendance: does the streak survive when few holders check in daily?

| Average daily check-in rate | Days hit, fixed 50% target | Days hit, retargeting | Best streak | Pool burned (% of max supply) |
|---|---|---|---|---|
| 37% | 0/60 | 59/60 | 59 | 0.01% |
| 19% | 0/60 | 59/60 | 59 | 0.01% |
| 9% | 0/60 | 57/60 | 57 | 0.04% |
| 4.5% | 0/60 | 49/60 | 48 | 0.14% |

The first design required half of all holders to check in every day. No real community does that, so the pool would burn almost every day. The contract now sets the bar at 75% of the community's own trailing-average turnout, which works at any attendance level and still punishes a real drop (E3).

### E3 Sudden exodus: a share of holders leaves on one day

| Holders who leave | Missed days after | Days until the streak restarts | Pool burned by it |
|---|---|---|---|
| 25% | 1 | 1 | 0.3% |
| 50% | 7 | 6 | 2.1% |
| 75% | 14 | 14 | 4.0% |

A small drop is absorbed. A large drop costs a run of missed days, each burning 0.25% of the pool, until the target has come down to the community that is left. There is no death spiral: the cost is bounded by the retarget window.

### E4 Faking a milestone: the creator rents 200 'holders' to unlock the 9% tranche

| Fake holders stay after the mint | Capital locked per fake holder | Creator earn-out kept | Clawed back | Reserve cancelled by the contraction |
|---|---|---|---|---|
| 0 days | 14 days | 4.20M (23% of the cut) | 13.80M | 39M |
| 7 days | 21 days | 8.40M (47% of the cut) | 9.60M | 39M |
| 14 days | 28 days | 12.60M (70% of the cut) | 5.40M | 39M |
| 30 days | 44 days | 18.00M (100% of the cut) | 0.00M | 39M |
| 60 days | 74 days | 18.00M (100% of the cut) | 0.00M | 39M |

Each fake holder must hold at least `holder_min_balance` with 14 days of coin-age, in a wallet whose coins did not mostly come from another counted wallet (the on-chain majority-source record). The creator's cut vests over 30 days and the unvested part is cancelled when the holders leave, together with 10% of the remaining reserve. Faking growth only pays if the fake holders stay for the whole vest, at which point they are capital held in the coin for six weeks.

### E5 Compromised oracle: every oracle key is stolen on day 0

| Day | Honest growth minted | Minted with no bounds | Minted with the contract's bounds | Of which the creator could have claimed |
|---|---|---|---|---|
| 0 | 0% | 70% | 5% | 0.0% |
| 7 | 0% | 70% | 12% | 0.2% |
| 14 | 0% | 70% | 21% | 0.7% |
| 30 | 21% | 70% | 44% | 2.9% |
| 60 | 34% | 70% | 70% | 9.3% |
| 90 | 36% | 70% | 70% | 14.0% |

Bounds enforced in the contract: m-of-n signatures, one tranche per `tranche_cooldown_days` (7 in production), the reported count can never exceed the on-chain holder count, and the creator's cut vests over 90 days. Even then 80% of every tranche goes to the pool, which pays the people who check in. A stolen oracle can mis-time supply; it cannot move a balance, change a rule or stop the ritual.

### E6 Wallet farm: one farmer holds 13% of the coins, never misses a day, and splits them across N wallets

| Wallets | Each wallet holds | Share of rewards, naive coin | INSYDR, none detected | INSYDR, 80% detected | INSYDR, all detected |
|---|---|---|---|---|---|
| 1 | 91.3x the cap | 0.7% | 1.1% | 1.1% | 1.1% |
| 30 | 3.0x the cap | 16.1% | 21.8% | 7.9% | 1.0% |
| 300 | below the cap | 63.8% | 52.7% | 21.0% | 0.3% |
| 3000 | below the cap | 94.6% | 55.2% | 24.9% | 0.0% |

On a naive coin every extra wallet is extra money, without limit. On INSYDR the weight is linear in aged stake, so once each wallet is under the whale cap, more wallets earn nothing more: the farm's ceiling is what the same coins earn in honest hands that show up every day. Splitting buys exactly one thing, getting around the whale cap, and that is what cluster detection (shared parent account, shared majority token source) takes away. Detected wallets fall to roughly nothing.

### E7 Challenge drain: the creator signs completions for wallets of their own

| Daily cap / per-account cap | Creator's undetected full-stake wallets | Pool taken per day | Pool taken in 90 days | Relative to the ritual's daily payout |
|---|---|---|---|---|
| 1.0% / 100% | 1 | 1.000% | 59.5% | 200% |
| 1.0% / 100% | 5 | 1.000% | 59.5% | 200% |
| 1.0% / 100% | 20 | 1.000% | 59.5% | 200% |
| 1.0% / 5% | 1 | 0.050% | 4.4% | 10% |
| 1.0% / 5% | 5 | 0.250% | 20.2% | 50% |
| 1.0% / 5% | 20 | 1.000% | 59.5% | 200% |
| 0.1% / 5% | 1 | 0.005% | 0.4% | 1% |
| 0.1% / 5% | 5 | 0.025% | 2.2% | 5% |
| 0.1% / 5% | 20 | 0.100% | 8.6% | 20% |

Challenges are the one place the creator has discretion, so they are boxed in: a daily cap for all challenges together, a per-account share of that cap, and the same anti-farm weight as the ritual (each wallet needs a full, aged stake to collect its share). Production values: 0.1% / 5%, so the worst case is a fifth of what the ritual pays the community each day, and it takes 20 fully staked undetected wallets to reach it.
