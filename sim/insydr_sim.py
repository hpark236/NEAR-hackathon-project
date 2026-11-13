"""
INSYDR community simulation (60 days, one tick = one day).

Purpose: choose contract parameters with evidence (the memecoins track prompt asks for a 60-day
simulation before picking parameters), and show how the anti-farm rules change who gets paid
compared with a naive streak coin. The rules here mirror contracts/insydr/src/lib.rs.

Rules modelled
  * Max supply 1,000,000,000. Launch: 35% sold, 5% seeds the community pool, 60% unminted reserve.
  * Growth tranches: when INDEPENDENT holders (clusters counted once, coin-age >= 14 days) cross
    a milestone, a tranche mints: 80% to the pool, 20% to the creator earn-out, which vests over
    `creator_vest_days`. At most one tranche per `tranche_cooldown_days`.
  * Contraction: below the last milestone for 7 days -> 10% of the reserve is cancelled and the
    creator's unvested earn-out is clawed back.
  * Daily ritual: turnout = sum of the anti-farm weights that checked in. The day is a hit when
    turnout >= `quorum_bps` of the trailing-average turnout (and at least `quorum_min` unique
    independent check-ins). Hit -> 0.5% of the pool is paid by weight. Miss -> 0.25% of the pool
    burns and the community streak resets. The target retargets every day (moving average).
  * Weight = independence share x coin-age ramp x capped linear stake, times a streak bonus of up
    to +50%. A missed day costs `streak_miss_penalty` days of streak instead of resetting it.
  * Season (30 days): +50% independent holders mints a 2% bonus to the pool, else it is cancelled.

Naive mode is the baseline to beat: every check-in weighs 1, the target is a fixed share of all
accounts, and a missed day resets the streak.

Actors
  organic  real people arriving over time. A quarter are regulars (check in 70-95% of days), the
           rest are casual (5-40%). A small daily chance to leave.
  insider  30 creator-side wallets holding 25x a normal bag. They check in daily and dump on day 12.
  pump     a pump group: 250 distinct humans who arrive days 4-8, check in daily, leave days 14-18.
  sybil    one farmer running 300 wallets that check in every day and never leave.

Output: CSVs and PNG charts in sim/results/. Standard library only, plus matplotlib for charts.
Deterministic seeds.
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
    tranche_cooldown_days=3,
    creator_vest_days=30,
    hold_days_for_milestone=14,
    contraction_grace_days=7,
    contraction_cut=0.10,
    quorum_min=10,
    quorum_frac=0.75,          # of the trailing-average turnout
    quorum_window_days=7,
    daily_payout_frac=0.005,
    miss_burn_frac=0.0025,
    age_full_days=14,
    streak_cap=30,
    streak_max_bonus=0.5,      # streak multiplier tops out at 1.5x
    streak_miss_penalty=1,     # days of streak lost per missed day
    balance_cap=0.0005,        # weight grows with balance up to 0.05% of max supply
    season_len=30,
    season_growth_goal=0.50,
    season_bonus=0.02,
    naive_quorum_frac=0.20,    # naive coin: a fixed share of all accounts must check in
)


@dataclass
class Account:
    kind: str            # organic | insider | pump | sybil
    joined: int
    leave_day: int | None
    checkin_p: float
    cluster: str | None  # detected cluster id, or None if it looks independent
    balance: float = 0.0
    start_balance: float = 0.0
    streak: int = 0
    last_checkin: int | None = None
    rewards: float = 0.0
    active: bool = False


@dataclass
class World:
    naive: bool
    rng: random.Random
    p: dict
    accounts: list[Account] = field(default_factory=list)
    pool: float = 0.0
    reserve: float = 0.0
    creator_unlocked: float = 0.0      # granted, net of clawbacks
    creator_vests: list = field(default_factory=list)   # [start_day, amount]
    burned: float = 0.0
    cancelled: float = 0.0
    milestone_idx: int = 0
    last_tranche_day: int | None = None
    below_days: int = 0
    community_streak: int = 0
    best_streak: int = 0
    days_hit: int = 0
    days_missed: int = 0
    turnout_avg: float = 0.0
    season_start_indep: int = 0

    def __post_init__(self):
        self.pool = MAX_SUPPLY * self.p["pool_seed"]
        self.reserve = MAX_SUPPLY * self.p["reserve"]

    @property
    def supply(self) -> float:
        # minted and not burned; the creator's unclaimed earn-out is not minted yet
        return MAX_SUPPLY - self.reserve - self.burned - self.cancelled - self.creator_unlocked

    def creator_vested(self, day: int) -> float:
        v = self.p["creator_vest_days"]
        return sum(a * (min(1.0, (day - s) / v) if v else 1.0) for s, a in self.creator_vests)


def independent_units(accts, day, min_hold=0):
    """Count independent holders: each detected cluster = 1, undetected accounts = 1 each."""
    clusters, singles = set(), 0
    for a in accts:
        if not a.active or day - a.joined < min_hold:
            continue
        if a.cluster:
            clusters.add(a.cluster)
        else:
            singles += 1
    return len(clusters) + singles


def organic_p(rng: random.Random, scale: float = 1.0) -> float:
    p = rng.uniform(0.70, 0.95) if rng.random() < 0.25 else rng.uniform(0.05, 0.40)
    return min(1.0, p * scale)


def build(scenario: str, naive: bool, seed: int, p: dict | None = None, attendance_scale: float = 1.0,
          sybil_detect: float = 0.8, sybil_n: int = 300, sybil_units: float = 90.0) -> World:
    rng = random.Random(seed)
    w = World(naive=naive, rng=rng, p=dict(P if p is None else p))
    buy = MAX_SUPPLY * w.p["launch_sold"]

    def add(kind, joined, leave, prob, cluster):
        w.accounts.append(Account(kind, joined, leave, prob, cluster))

    target = {"organic": 900, "insider_pump": 600, "sybil_farm": 600, "dead": 120}[scenario]
    for _ in range(target):
        if scenario == "dead":
            day = int(rng.random() * 20)          # a small burst, then nothing
        else:
            u = min(max(rng.random(), 1e-6), 1 - 1e-6)
            day = int(min(DAYS - 1, max(0, 22 + 8 * math.log(u / (1 - u)))))  # logistic S-curve
        add("organic", day, None, organic_p(rng, attendance_scale), None)
    if scenario == "dead":
        for a in w.accounts:
            a.leave_day = rng.randint(22, 40)     # ...and then they drift away

    if scenario == "insider_pump":
        for _ in range(30):
            add("insider", 0, 12, 1.0, "insider" if rng.random() < 0.9 else None)
        for _ in range(250):
            add("pump", rng.randint(4, 8), rng.randint(14, 18), 0.95, None)
    if scenario == "sybil_farm":
        for _ in range(sybil_n):
            add("sybil", rng.randint(0, 3), None, 1.0, "farm" if rng.random() < sybil_detect else None)

    # bag sizes relative to one organic holder; the farm splits `sybil_units` bags across its wallets
    weights = [{"organic": 1.0, "insider": 25.0, "pump": 2.0, "sybil": sybil_units / sybil_n}[a.kind] for a in w.accounts]
    tot = sum(weights)
    for a, wt in zip(w.accounts, weights):
        a.balance = a.start_balance = buy * wt / tot
    return w


def base_weight(w: World, a: Account, day: int, cluster_sizes) -> float:
    """independence share x coin-age ramp x capped linear stake (no streak bonus)."""
    if w.naive:
        return 1.0
    p = w.p
    indep = 1.0 / cluster_sizes[a.cluster] if a.cluster else 1.0
    age = min(1.0, (day - a.joined) / p["age_full_days"])
    cap = p["balance_cap"] * MAX_SUPPLY
    return indep * age * min(a.balance, cap) / cap


def streak_factor(w: World, a: Account) -> float:
    if w.naive:
        return 1.0
    p = w.p
    return 1.0 + p["streak_max_bonus"] * min(a.streak, p["streak_cap"]) / p["streak_cap"]


def step(w: World, day: int):
    rng, p = w.rng, w.p
    for a in w.accounts:
        if a.joined == day:
            a.active = True
        if a.leave_day is not None and day >= a.leave_day and a.active:
            a.active = False           # sold out
        if a.kind == "organic" and a.active and day > a.joined and rng.random() < 0.004:
            a.active = False           # ordinary churn

    live = [a for a in w.accounts if a.active]
    if w.naive:
        indep_held = sum(1 for a in live if day - a.joined >= p["hold_days_for_milestone"])
    else:
        indep_held = independent_units(live, day, p["hold_days_for_milestone"])

    # growth tranches (rate-limited), creator cut vests
    ms = p["milestones"]
    if w.milestone_idx < len(ms) and indep_held >= ms[w.milestone_idx]:
        cooled = w.last_tranche_day is None or day - w.last_tranche_day >= p["tranche_cooldown_days"]
        if cooled or w.naive:
            amt = min(w.reserve, MAX_SUPPLY * p["tranche_share"][w.milestone_idx])
            w.reserve -= amt
            w.pool += amt * p["tranche_to_pool"]
            cut = amt * (1 - p["tranche_to_pool"])
            w.creator_unlocked += cut
            w.creator_vests.append([day, cut])
            w.milestone_idx += 1
            w.last_tranche_day = day
        w.below_days = 0
    elif w.milestone_idx > 0 and indep_held < ms[w.milestone_idx - 1]:
        w.below_days += 1
        if w.below_days >= p["contraction_grace_days"]:
            cut = w.reserve * p["contraction_cut"]
            w.reserve -= cut
            w.cancelled += cut
            if not w.naive:        # claw back the creator's unvested earn-out
                v = p["creator_vest_days"]
                for e in w.creator_vests:
                    vested = e[1] * (min(1.0, (day - e[0]) / v) if v else 1.0)
                    w.creator_unlocked -= e[1] - vested
                    w.cancelled += e[1] - vested
                    e[1] = vested
                    e[0] = day - v   # fully vested from here on
            w.below_days = 0
    else:
        w.below_days = 0

    # daily check-ins
    checked = [a for a in live if rng.random() < a.checkin_p]
    for a in checked:
        if w.naive:
            a.streak = a.streak + 1 if a.last_checkin == day - 1 else 1
        else:
            missed = 0 if a.last_checkin is None else day - a.last_checkin - 1
            a.streak = max(0, a.streak - missed * p["streak_miss_penalty"]) + 1
        a.last_checkin = day
    sizes = {}
    for a in checked:
        if a.cluster:
            sizes[a.cluster] = sizes.get(a.cluster, 0) + 1
    base = [base_weight(w, a, day, sizes) for a in checked]
    turnout = sum(base)
    if w.naive:
        hit = len(checked) >= max(p["quorum_min"], int(p["naive_quorum_frac"] * len(live)))
    else:
        unique = independent_units(checked, day)
        hit = unique >= p["quorum_min"] and turnout >= p["quorum_frac"] * w.turnout_avg and turnout > 0
        n = p["quorum_window_days"]
        w.turnout_avg = turnout if w.turnout_avg == 0 else (w.turnout_avg * (n - 1) + turnout) / n
    if hit and checked:
        w.community_streak += 1
        w.best_streak = max(w.best_streak, w.community_streak)
        w.days_hit += 1
        ws = [b * streak_factor(w, a) for a, b in zip(checked, base)]
        tw = sum(ws)
        payout = w.pool * p["daily_payout_frac"]
        if tw > 0:
            w.pool -= payout
            for a, x in zip(checked, ws):
                r = payout * x / tw
                a.rewards += r
                a.balance += r
    else:
        burn = w.pool * p["miss_burn_frac"]
        w.pool -= burn
        w.burned += burn
        w.community_streak = 0
        w.days_missed += 1

    # seasons
    if day % p["season_len"] == 0:
        if day > 0:
            goal = max(10, int(w.season_start_indep * (1 + p["season_growth_goal"])))
            bonus = min(w.reserve, MAX_SUPPLY * p["season_bonus"])
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
        cancelled=w.cancelled, creator=w.creator_unlocked, creator_vested=w.creator_vested(day),
        indep_holders=indep_held, live_accounts=len(live), checkins=len(checked),
        quorum_hit=int(hit), community_streak=w.community_streak,
        **{f"rewards_{k}": v for k, v in by_kind.items()},
    )


def run(scenario, naive, seed=7, days=DAYS, **kw):
    w = build(scenario, naive, seed, **kw)
    rows = [step(w, d) for d in range(days)]
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


# chart styling: one palette, fixed order, light surface (validated for colour-blind separation)
C = dict(blue="#2a78d6", orange="#eb6834", aqua="#1baf7a", yellow="#eda100", ink="#1f1f1d", muted="#6b6a63", grid="#e6e4dc")


def style(ax, title=None):
    ax.set_facecolor("white")
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    for s in ("left", "bottom"):
        ax.spines[s].set_color(C["grid"])
    ax.tick_params(colors=C["muted"], labelsize=9)
    ax.grid(axis="y", color=C["grid"], linewidth=0.8)
    ax.set_axisbelow(True)
    if title:
        ax.set_title(title, loc="left", fontsize=11, color=C["ink"], fontweight="bold")


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
                # two decimals: identical output on every Python version (float noise differs between releases)
                wr.writerows([{k: (round(v, 2) if isinstance(v, float) else v) for k, v in r.items()} for r in rows])
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
                creator_vested_pct=round(last["creator_vested"] / MAX_SUPPLY * 100, 2),
                cancelled_pct=round(last["cancelled"] / MAX_SUPPLY * 100, 1),
                burned_pct=round(last["burned"] / MAX_SUPPLY * 100, 2),
                indep_holders=last["indep_holders"], best_streak=w.best_streak,
                days_hit=w.days_hit, days_missed=w.days_missed,
            ))
    keys = ["scenario", "mode", "actor", "accounts", "reward_share", "reward_per_account", "reward_per_token_held",
            "final_supply_pct", "creator_vested_pct", "cancelled_pct", "burned_pct", "indep_holders", "best_streak",
            "days_hit", "days_missed"]
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

    # supply over time and the community that drives it: two rows, one measure per axis
    fig, axes = plt.subplots(2, len(scenarios), figsize=(17, 6.2), sharex=True, sharey="row")
    titles = {"organic": "Organic growth", "insider_pump": "Insiders + pump group",
              "sybil_farm": "300-wallet farm", "dead": "Community that fades"}
    for i, sc in enumerate(scenarios):
        rows = series[(sc, "insydr")]
        d = [r["day"] for r in rows]
        style(axes[0][i], titles[sc])
        axes[0][i].plot(d, [r["supply"] / 1e6 for r in rows], color=C["blue"], linewidth=2)
        style(axes[1][i])
        axes[1][i].plot(d, [r["indep_holders"] for r in rows], color=C["aqua"], linewidth=2)
        axes[1][i].set_xlabel("day", color=C["muted"], fontsize=9)
    axes[0][0].set_ylabel("supply (millions)", color=C["muted"], fontsize=9)
    axes[1][0].set_ylabel("independent holders\n(coin-age ≥ 14 days)", color=C["muted"], fontsize=9)
    fig.suptitle("INSYDR: supply follows the real community (60 days, 4 communities)", x=0.01, ha="left",
                 fontsize=13, fontweight="bold", color=C["ink"])
    fig.tight_layout()
    fig.savefig(os.path.join(res, "supply_vs_community.png"), dpi=130, facecolor="white")

    fig, ax = plt.subplots(figsize=(9, 4.4))
    style(ax, "Share of all rewards captured by bad actors")
    labels, naive_v, ins_v = [], [], []
    for sc, actor, lab in [("insider_pump", "insider", "30 insider wallets"), ("insider_pump", "pump", "250-person pump group"),
                           ("sybil_farm", "sybil", "300-wallet farm")]:
        get = lambda mode: next(r["reward_share"] for r in summary_rows
                                if r["scenario"] == sc and r["mode"] == mode and r["actor"] == actor)
        labels.append(lab)
        naive_v.append(get("naive") * 100)
        ins_v.append(get("insydr") * 100)
    x = range(len(labels))
    b1 = ax.bar([i - 0.2 for i in x], naive_v, 0.38, label="naive streak coin", color=C["orange"])
    b2 = ax.bar([i + 0.2 for i in x], ins_v, 0.38, label="INSYDR", color=C["blue"])
    for bars in (b1, b2):
        for b in bars:
            ax.text(b.get_x() + b.get_width() / 2, b.get_height() + 0.6, f"{b.get_height():.1f}%", ha="center",
                    fontsize=9, color=C["ink"])
    ax.set_xticks(list(x))
    ax.set_xticklabels(labels, color=C["ink"])
    ax.set_ylabel("% of all rewards", color=C["muted"], fontsize=9)
    ax.legend(frameon=False, fontsize=9)
    fig.tight_layout()
    fig.savefig(os.path.join(res, "farmer_capture.png"), dpi=130, facecolor="white")
    print("charts written to", res)


if __name__ == "__main__":
    main()
