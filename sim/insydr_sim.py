"""
INSYDR 60-day community simulation.

Purpose: choose contract parameters before writing the contract (the memecoins
track prompt asks for exactly this), and show how the anti-farm rules change who
gets paid compared with a naive streak coin.

Model (one tick = one day). The rules mirror the planned contract:
  * Max supply 1,000,000,000. At launch: 35% sold (auction/LP), 5% seeds the
    community pool, 60% sits unminted in the growth reserve.
  * Growth tranches: when the count of INDEPENDENT holders who have held >= 7
    days crosses a milestone, a tranche of the reserve mints: 80% to the
    community pool, 20% to the creator earn-out.
  * Contraction: if independent holders stay below the last milestone for 7
    days in a row, 10% of the remaining reserve is cancelled for good.
  * Daily streak: quorum = max(10, 20% of independent holders). Hit -> 0.5% of
    the pool is paid to today's check-ins by weight. Miss -> 0.25% of the pool
    burns and the community streak resets.
  * Weight per account = independence share x holding-age factor x streak factor.
      independence share: a detected cluster of k wallets shares one weight.
      age factor: min(1, days held / 14).
      streak factor: 1 + min(personal streak, 30) / 30   (up to 2x)
    Naive mode (the baseline to beat) sets every weight to 1.
  * Season (30 days): goal = +50% independent holders. Hit -> a bonus tranche
    (2% of max supply) goes to the pool; miss -> that tranche is cancelled.

Actors:
  organic  - real people arriving over time, each with their own check-in habit
             and a small daily chance to leave.
  insider  - 30 wallets funded by the deployer (detected as one cluster with
             probability 0.9 each); they check in daily and dump on day 12.
  pump     - a Telegram pump group: 250 distinct humans (NOT clusterable) who
             arrive days 4-8, check in daily, and leave days 14-18.
  sybil    - one farmer running 300 accounts (each detected with probability
             0.8); they check in every day and never leave.

Scenarios mix these actors. Output: CSVs + PNG charts in sim/results/.
Pure standard library except matplotlib (charts only). Deterministic seeds.
"""
from __future__ import annotations

import csv
import math
import os
import random
from dataclasses import dataclass, field

MAX_SUPPLY = 1_000_000_000
DAYS = 60

P = dict(
    launch_sold=0.35,
    pool_seed=0.05,
    reserve=0.60,
    milestones=[50, 100, 200, 400, 800, 1600, 3200],
    tranche_share=[0.05, 0.07, 0.09, 0.11, 0.12, 0.13, 0.13],  # of max supply
    tranche_to_pool=0.80,
    hold_days_for_milestone=14,
    contraction_grace_days=7,
    contraction_cut=0.10,
    quorum_min=10,
    quorum_frac=0.50,
    daily_payout_frac=0.005,
    miss_burn_frac=0.0025,
    age_full_days=14,
    streak_cap=30,
    streak_max_bonus=0.5,      # streak multiplier tops out at 1.5x
    balance_cap=0.0005,        # reward weight grows with balance up to 0.05% of max supply
    season_len=30,
    season_growth_goal=0.50,
    season_bonus=0.02,
)


@dataclass
class Account:
    kind: str           # organic | insider | pump | sybil
    joined: int
    leave_day: int | None
    checkin_p: float
    cluster: str | None  # detected cluster id, or None if it looks independent
    balance: float = 0.0
    start_balance: float = 0.0
    streak: int = 0
    rewards: float = 0.0
    active: bool = True


@dataclass
class World:
    naive: bool
    rng: random.Random
    accounts: list[Account] = field(default_factory=list)
    pool: float = MAX_SUPPLY * P["pool_seed"]
    reserve: float = MAX_SUPPLY * P["reserve"]
    creator_unlocked: float = 0.0
    burned: float = 0.0
    cancelled: float = 0.0
    milestone_idx: int = 0
    below_days: int = 0
    community_streak: int = 0
    best_streak: int = 0
    season_start_indep: int = 0

    @property
    def supply(self) -> float:
        # minted and not burned (pool + creator + holders)
        return MAX_SUPPLY - self.reserve - self.burned - self.cancelled


def independent_units(accts, day, min_hold=0):
    """Count independent holders: each detected cluster = 1, undetected = 1 each."""
    clusters, singles = set(), 0
    for a in accts:
        if not a.active or day - a.joined < min_hold:
            continue
        if a.cluster:
            clusters.add(a.cluster)
        else:
            singles += 1
    return len(clusters) + singles


def build(scenario: str, naive: bool, seed: int) -> World:
    rng = random.Random(seed)
    w = World(naive=naive, rng=rng)
    buy = MAX_SUPPLY * P["launch_sold"]

    def add(kind, joined, leave, p, cluster):
        w.accounts.append(Account(kind, joined, leave, p, cluster))

    # organic arrivals: S-curve towards a target audience size
    target = {"organic": 900, "insider_pump": 600, "sybil_farm": 600, "dead": 120}[scenario]
    for i in range(target):
        # logistic arrival day
        if scenario == "dead":
            day = int(rng.random() * 20)          # a small burst, then nothing
        else:
            u = min(max(rng.random(), 1e-6), 1 - 1e-6)
            day = int(min(DAYS - 1, max(0, 22 + 8 * math.log(u / (1 - u)))))  # logistic S-curve
        add("organic", day, None, rng.uniform(0.35, 0.9), None)

    if scenario == "insider_pump":
        for i in range(30):
            add("insider", 0, 12, 1.0, "insider" if rng.random() < 0.9 else None)
        for i in range(250):
            add("pump", rng.randint(4, 8), rng.randint(14, 18), 0.95, None)
    if scenario == "sybil_farm":
        for i in range(300):
            add("sybil", rng.randint(0, 3), None, 1.0, "farm" if rng.random() < 0.8 else None)

    # initial balances: sold supply split by kind (insiders front-run the launch)
    weights = []
    for a in w.accounts:
        weights.append({"organic": 1.0, "insider": 25.0, "pump": 2.0, "sybil": 0.3}[a.kind])
    tot = sum(weights)
    for a, wt in zip(w.accounts, weights):
        a.balance = a.start_balance = buy * wt / tot
        a.active = a.joined == 0
    return w


def weight(w: World, a: Account, day: int, cluster_sizes) -> float:
    if w.naive:
        return 1.0
    indep = 1.0 / cluster_sizes[a.cluster] if a.cluster else 1.0
    age = min(1.0, (day - a.joined) / P["age_full_days"])
    streak = 1.0 + P["streak_max_bonus"] * min(a.streak, P["streak_cap"]) / P["streak_cap"]
    # capped linear in balance: splitting a stack across many accounts earns nothing extra
    stake = min(a.balance, P["balance_cap"] * MAX_SUPPLY) / (P["balance_cap"] * MAX_SUPPLY)
    return indep * age * streak * stake


def step(w: World, day: int):
    rng = w.rng
    for a in w.accounts:
        if a.joined == day:
            a.active = True
        if a.leave_day is not None and day >= a.leave_day and a.active:
            a.active = False           # sold out
        if a.kind == "organic" and a.active and day > a.joined and rng.random() < 0.004:
            a.active = False           # ordinary churn

    live = [a for a in w.accounts if a.active]
    if w.naive:
        indep_all = len(live)
        indep_held = sum(1 for a in live if day - a.joined >= P["hold_days_for_milestone"])
    else:
        indep_all = independent_units(live, day)
        indep_held = independent_units(live, day, P["hold_days_for_milestone"])

    # growth tranches
    ms = P["milestones"]
    while w.milestone_idx < len(ms) and indep_held >= ms[w.milestone_idx]:
        amt = min(w.reserve, MAX_SUPPLY * P["tranche_share"][w.milestone_idx])
        w.reserve -= amt
        w.pool += amt * P["tranche_to_pool"]
        w.creator_unlocked += amt * (1 - P["tranche_to_pool"])
        w.milestone_idx += 1
        w.below_days = 0
    # contraction
    if w.milestone_idx > 0 and indep_held < ms[w.milestone_idx - 1]:
        w.below_days += 1
        if w.below_days >= P["contraction_grace_days"]:
            cut = w.reserve * P["contraction_cut"]
            w.reserve -= cut
            w.cancelled += cut
            w.below_days = 0
    else:
        w.below_days = 0

    # daily check-ins
    checked = [a for a in live if rng.random() < a.checkin_p]
    for a in live:
        a.streak = a.streak + 1 if a in checked else 0
    quorum = max(P["quorum_min"], int(P["quorum_frac"] * indep_all))
    hit = (len(checked) if w.naive else independent_units(checked, day)) >= quorum
    if hit and checked:
        w.community_streak += 1
        w.best_streak = max(w.best_streak, w.community_streak)
        sizes = {}
        for a in checked:
            if a.cluster:
                sizes[a.cluster] = sizes.get(a.cluster, 0) + 1
        ws = [weight(w, a, day, sizes) for a in checked]
        tw = sum(ws)
        payout = w.pool * P["daily_payout_frac"]
        if tw > 0:
            w.pool -= payout
            for a, x in zip(checked, ws):
                r = payout * x / tw
                a.rewards += r
                a.balance += r
    else:
        burn = w.pool * P["miss_burn_frac"]
        w.pool -= burn
        w.burned += burn
        w.community_streak = 0

    # seasons
    if day % P["season_len"] == 0:
        if day > 0:
            goal = max(10, int(w.season_start_indep * (1 + P["season_growth_goal"])))
            bonus = min(w.reserve, MAX_SUPPLY * P["season_bonus"])
            w.reserve -= bonus
            if indep_held >= goal:
                w.pool += bonus
            else:
                w.cancelled += bonus
        w.season_start_indep = indep_held

    by_kind = {}
    for a in w.accounts:
        by_kind[a.kind] = by_kind.get(a.kind, 0.0) + a.rewards
    return dict(
        day=day, supply=w.supply, pool=w.pool, reserve=w.reserve, burned=w.burned,
        cancelled=w.cancelled, creator=w.creator_unlocked, indep_holders=indep_held,
        live_accounts=len(live), quorum_hit=int(hit), community_streak=w.community_streak,
        **{f"rewards_{k}": v for k, v in by_kind.items()},
    )


def run(scenario, naive, seed=7):
    w = build(scenario, naive, seed)
    rows = [step(w, d) for d in range(DAYS)]
    return w, rows


def summarize(w: World):
    tot = sum(a.rewards for a in w.accounts) or 1.0
    out = {}
    for kind in ("organic", "insider", "pump", "sybil"):
        accts = [a for a in w.accounts if a.kind == kind]
        if accts:
            r = sum(a.rewards for a in accts)
            held = sum(a.start_balance for a in accts) or 1.0
            out[kind] = dict(n=len(accts), share=r / tot, per_account=r / len(accts), per_token=r / held)
    return out


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    res = os.path.join(here, "results")
    os.makedirs(res, exist_ok=True)
    scenarios = ["organic", "insider_pump", "sybil_farm", "dead"]
    summary_rows, series = [], {}
    for sc in scenarios:
        for naive in (True, False):
            w, rows = run(sc, naive)
            mode = "naive" if naive else "insydr"
            series[(sc, mode)] = rows
            with open(os.path.join(res, f"{sc}_{mode}.csv"), "w", newline="") as f:
                wr = csv.DictWriter(f, fieldnames=sorted({k for r in rows for k in r}, key=lambda k: (k != "day", k)))
                wr.writeheader()
                wr.writerows(rows)
            for kind, s in summarize(w).items():
                summary_rows.append(dict(
                    scenario=sc, mode=mode, actor=kind, accounts=s["n"],
                    reward_share=round(s["share"], 4),
                    reward_per_account=round(s["per_account"]),
                    reward_per_token_held=round(s["per_token"], 4),
                ))
            last = rows[-1]
            summary_rows.append(dict(
                scenario=sc, mode=mode, actor="(coin)", accounts=last["live_accounts"],
                reward_share="", reward_per_account="",
                final_supply_pct=round(last["supply"] / MAX_SUPPLY * 100, 1),
                creator_unlocked_pct=round(last["creator"] / MAX_SUPPLY * 100, 2),
                cancelled_pct=round(last["cancelled"] / MAX_SUPPLY * 100, 1),
                indep_holders=last["indep_holders"], best_streak=w.best_streak,
            ))
    keys = ["scenario", "mode", "actor", "accounts", "reward_share", "reward_per_account", "reward_per_token_held",
            "final_supply_pct", "creator_unlocked_pct", "cancelled_pct", "indep_holders", "best_streak"]
    with open(os.path.join(res, "summary.csv"), "w", newline="") as f:
        wr = csv.DictWriter(f, fieldnames=keys)
        wr.writeheader()
        wr.writerows(summary_rows)
    for r in summary_rows:
        print({k: r.get(k) for k in keys if r.get(k) not in (None, "")})

    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not installed; skipped charts")
        return

    fig, axes = plt.subplots(1, len(scenarios), figsize=(18, 4.2), sharey=True)
    for ax, sc in zip(axes, scenarios):
        rows = series[(sc, "insydr")]
        d = [r["day"] for r in rows]
        ax.plot(d, [r["supply"] / 1e6 for r in rows], label="circulating+pool supply (M)")
        ax2 = ax.twinx()
        ax2.plot(d, [r["indep_holders"] for r in rows], color="tab:green", ls="--", label="independent holders (held ≥14d)")
        ax.set_title(sc.replace("_", " + "))
        ax.set_xlabel("day")
        if sc == scenarios[0]:
            ax.set_ylabel("supply (millions)")
        if sc == scenarios[-1]:
            ax2.set_ylabel("independent holders")
    axes[0].legend(loc="upper left", fontsize=8)
    fig.suptitle("INSYDR: supply follows the real community")
    fig.tight_layout()
    fig.savefig(os.path.join(res, "supply_vs_community.png"), dpi=120)

    fig, ax = plt.subplots(figsize=(9, 4.2))
    labels, naive_v, grass_v = [], [], []
    for sc, actor in [("insider_pump", "insider"), ("insider_pump", "pump"), ("sybil_farm", "sybil")]:
        get = lambda mode: next(r["reward_share"] for r in summary_rows
                                if r["scenario"] == sc and r["mode"] == mode and r["actor"] == actor)
        labels.append(f"{actor}\n({sc.replace('_', ' + ')})")
        naive_v.append(get("naive") * 100)
        grass_v.append(get("insydr") * 100)
    x = range(len(labels))
    ax.bar([i - 0.2 for i in x], naive_v, 0.4, label="naive streak coin")
    ax.bar([i + 0.2 for i in x], grass_v, 0.4, label="INSYDR")
    ax.set_xticks(list(x))
    ax.set_xticklabels(labels)
    ax.set_ylabel("% of all rewards captured")
    ax.set_title("Who captures the rewards: bad actors, naive vs INSYDR")
    ax.legend()
    fig.tight_layout()
    fig.savefig(os.path.join(res, "farmer_capture.png"), dpi=120)
    print("charts written to", res)


if __name__ == "__main__":
    main()
