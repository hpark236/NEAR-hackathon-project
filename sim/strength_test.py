"""
INSYDR strength test: seven attacks and stress cases, each with numbers.

Run:  python sim/strength_test.py      (writes sim/results/strength/*.csv, *.png and report.md)

Every experiment uses the same model as sim/insydr_sim.py, or a closed-form bound that follows
directly from a contract rule (the rule and the unit test that pins it are named in the report).
"""
from __future__ import annotations

import csv
import os
import random

from insydr_sim import C, DAYS, MAX_SUPPLY, P, build, independent_units, run, step, style, summarize

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "results", "strength")
os.makedirs(OUT, exist_ok=True)
REPORT: list[str] = []


def table(title: str, header: list[str], rows: list[list], note: str = ""):
    REPORT.append(f"### {title}\n")
    REPORT.append("| " + " | ".join(header) + " |")
    REPORT.append("|" + "|".join("---" for _ in header) + "|")
    for r in rows:
        REPORT.append("| " + " | ".join(str(x) for x in r) + " |")
    if note:
        REPORT.append("\n" + note)
    REPORT.append("")
    name = title.split(":")[0].lower().replace(" ", "_")
    with open(os.path.join(OUT, f"{name}.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(header)
        w.writerows(rows)
    print(f"\n{title}")
    for r in rows:
        print("  ", dict(zip(header, r)))


# ---------------------------------------------------------------- E1: bots vs people (streaks)
def e1_streak_fairness():
    """Per-token reward of a cron job (100% attendance) relative to a person, by streak rule."""
    def mean_multiplier(attend: float, penalty: int | None, days=365, seed=1):
        rng = random.Random(seed)
        streak, last, total = 0, None, 0.0
        for d in range(days):
            if rng.random() < attend:
                if penalty is None:                       # reset rule
                    streak = streak + 1 if last == d - 1 else 1
                else:                                     # decay rule
                    missed = 0 if last is None else d - last - 1
                    streak = max(0, streak - missed * penalty) + 1
                last = d
                if d >= 90:                               # steady state only
                    total += 1 + P["streak_max_bonus"] * min(streak, P["streak_cap"]) / P["streak_cap"]
        return total / (days - 90)   # expected daily weight per token (attendance x multiplier)

    rows = []
    for attend in (0.9, 0.8, 0.6, 0.4):
        bot = {k: mean_multiplier(1.0, v) for k, v in (("reset", None), ("p1", 1), ("p2", 2), ("p3", 3))}
        hum = {k: sum(mean_multiplier(attend, v, seed=s) for s in range(20)) / 20
               for k, v in (("reset", None), ("p1", 1), ("p2", 2), ("p3", 3))}
        rows.append([f"{int(attend * 100)}%", f"{1 / attend:.2f}x"] + [f"{bot[k] / hum[k]:.2f}x" for k in ("reset", "p3", "p2", "p1")])
    table("E1 Bots vs people: how much more a cron job earns per token than a person",
          ["Person's attendance", "Attendance alone", "Streak resets on a miss", "Decay, penalty 3", "Decay, penalty 2", "Decay, penalty 1"],
          rows,
          "A wallet that never misses will always out-earn one that does. The question is by how much. "
          "With reset-on-miss, the streak bonus multiplies the gap. With decay (the contract's rule, "
          "`streak_miss_penalty`), the gap falls back to plain attendance for anyone who shows up most days. "
          "Chosen: penalty 1 (miss a day, lose a day). Below roughly 50% attendance no streak rule helps: "
          "the bonus is capped at 1.5x, so that is also the most a script can ever gain from it.")


# ---------------------------------------------------------------- E2: realistic attendance
def e2_attendance():
    """Days hit under a fixed-share target (old rule) vs the retargeting turnout rule (new)."""
    rows = []
    for scale, label in ((1.0, "37%"), (0.5, "19%"), (0.25, "9%"), (0.12, "4.5%")):
        # old rule: unique independent check-ins >= 50% of independent holders
        w = build("organic", False, 7, attendance_scale=scale)
        hits_old = 0
        for d in range(DAYS):
            live_before = None
            r = step(w, d)
            live = [a for a in w.accounts if a.active]
            indep = independent_units(live, d)
            hits_old += int(r["checkins"] >= max(P["quorum_min"], 0.5 * indep))
        w2, rows2 = run("organic", False, 7, attendance_scale=scale)
        rows.append([label, f"{hits_old}/60", f"{w2.days_hit}/60", w2.best_streak, f"{w2.burned / MAX_SUPPLY * 100:.2f}%"])
    table("E2 Realistic attendance: does the streak survive when few holders check in daily?",
          ["Average daily check-in rate", "Days hit, fixed 50% target", "Days hit, retargeting", "Best streak", "Pool burned (% of max supply)"],
          rows,
          "The first design required half of all holders to check in every day. No real community does that, "
          "so the pool would burn almost every day. The contract now sets the bar at 75% of the community's own "
          "trailing-average turnout, which works at any attendance level and still punishes a real drop (E3).")


# ---------------------------------------------------------------- E3: sudden exodus
def e3_exodus():
    rows = []
    for frac in (0.25, 0.5, 0.75):
        w = build("organic", False, 11)
        for a in w.accounts:                       # an established community from day 0
            a.joined = 0
        rng = random.Random(3)
        misses_after, burned_before = 0, 0.0
        recovered = None
        for d in range(90):
            if d == 45:
                burned_before = w.burned
                for a in w.accounts:
                    if a.active and rng.random() < frac:
                        a.leave_day = 45
            r = step(w, d)
            if d >= 45:
                if not r["quorum_hit"]:
                    misses_after += 1
                elif recovered is None and misses_after > 0:
                    recovered = d - 45
        rows.append([f"{int(frac * 100)}%", misses_after, recovered if recovered is not None else "0",
                     f"{(w.burned - burned_before) / (w.pool + w.burned - burned_before) * 100:.1f}%"])
    table("E3 Sudden exodus: a share of holders leaves on one day",
          ["Holders who leave", "Missed days after", "Days until the streak restarts", "Pool burned by it"],
          rows,
          "A small drop is absorbed. A large drop costs a run of missed days, each burning 0.25% of the pool, "
          "until the target has come down to the community that is left. There is no death spiral: the cost is "
          "bounded by the retarget window.")


# ---------------------------------------------------------------- E4: faking a milestone
def e4_fake_milestone():
    """What the creator gets for renting fake holders, by how long they stay (closed form)."""
    p = P
    tranche = MAX_SUPPLY * p["tranche_share"][2]              # the 200-holder tranche: 9% of supply
    cut = tranche * (1 - p["tranche_to_pool"])                # creator's 20%
    rows = []
    for stay in (0, 7, 14, 30, 60):
        # holders must already have 14 days of coin-age to count. They then stay `stay` more days.
        # After they leave: 7 days below the milestone, then contraction claws back what is unvested.
        vest_days = min(p["creator_vest_days"], stay + p["contraction_grace_days"])
        kept = cut * vest_days / p["creator_vest_days"]
        capital_days = 14 + stay
        rows.append([f"{stay} days", f"{capital_days} days", f"{kept / 1e6:.2f}M ({kept / cut * 100:.0f}% of the cut)",
                     f"{(cut - kept) / 1e6:.2f}M", f"{MAX_SUPPLY * (p['reserve'] - sum(p['tranche_share'][:3])) * p['contraction_cut'] / 1e6:.0f}M"])
    table("E4 Faking a milestone: the creator rents 200 'holders' to unlock the 9% tranche",
          ["Fake holders stay after the mint", "Capital locked per fake holder", "Creator earn-out kept", "Clawed back", "Reserve cancelled by the contraction"],
          rows,
          "Each fake holder must hold at least `holder_min_balance` with 14 days of coin-age, in a wallet whose coins "
          "did not mostly come from another counted wallet (the on-chain majority-source record). The creator's cut "
          "vests over 30 days and the unvested part is cancelled when the holders leave, together with 10% of the "
          "remaining reserve. Faking growth only pays if the fake holders stay for the whole vest, at which point "
          "they are capital held in the coin for six weeks.")


# ---------------------------------------------------------------- E5: compromised oracle
def e5_oracle():
    p = P
    rows = []
    for day in (0, 7, 14, 30, 60, 90):
        # honest organic growth for comparison
        honest_w, honest_rows = run("organic", False, 7, days=max(day, 1))
        honest = (MAX_SUPPLY * p["reserve"] - honest_w.reserve - honest_w.cancelled) / MAX_SUPPLY
        # a fully compromised oracle (all keys) claims every milestone at once: one tranche per cooldown
        n = min(len(p["tranche_share"]), 1 + day // 7)
        no_limit = sum(p["tranche_share"])
        limited = sum(p["tranche_share"][:n])
        rows.append([day, f"{honest * 100:.0f}%", f"{no_limit * 100:.0f}%", f"{limited * 100:.0f}%",
                     f"{limited * (1 - p['tranche_to_pool']) * min(1, day / 90) * 100:.1f}%"])
    table("E5 Compromised oracle: every oracle key is stolen on day 0",
          ["Day", "Honest growth minted", "Minted with no bounds", "Minted with the contract's bounds", "Of which the creator could have claimed"],
          rows,
          "Bounds enforced in the contract: m-of-n signatures, one tranche per `tranche_cooldown_days` (7 in production), "
          "the reported count can never exceed the on-chain holder count, and the creator's cut vests over 90 days. "
          "Even then 80% of every tranche goes to the pool, which pays the people who check in. A stolen oracle can "
          "mis-time supply; it cannot move a balance, change a rule or stop the ritual.")


# ---------------------------------------------------------------- E6: sybil farm tiers
def e6_sybil():
    """One farmer holds 13% of the coins and never misses a day. Does splitting them pay?"""
    rows = []
    cap = P["balance_cap"] * MAX_SUPPLY
    for n in (1, 30, 300, 3000):
        out = {}
        for key, naive, detect in (("naive", True, 0.0), ("none", False, 0.0), ("most", False, 0.8), ("all", False, 1.0)):
            w, _ = run("sybil_farm", naive, 7, sybil_detect=detect, sybil_n=n)
            out[key] = summarize(w)["sybil"]["share"]
        per_wallet = next(a.start_balance for a in w.accounts if a.kind == "sybil")
        rows.append([n, f"{per_wallet / cap:.1f}x the cap" if per_wallet > cap else "below the cap",
                     f"{out['naive'] * 100:.1f}%", f"{out['none'] * 100:.1f}%", f"{out['most'] * 100:.1f}%", f"{out['all'] * 100:.1f}%"])
    table("E6 Wallet farm: one farmer holds 13% of the coins, never misses a day, and splits them across N wallets",
          ["Wallets", "Each wallet holds", "Share of rewards, naive coin", "INSYDR, none detected", "INSYDR, 80% detected", "INSYDR, all detected"],
          rows,
          "On a naive coin every extra wallet is extra money, without limit. On INSYDR the weight is linear in "
          "aged stake, so once each wallet is under the whale cap, more wallets earn nothing more: the farm's "
          "ceiling is what the same coins earn in honest hands that show up every day. Splitting buys exactly "
          "one thing, getting around the whale cap, and that is what cluster detection (shared parent account, "
          "shared majority token source) takes away. Detected wallets fall to roughly nothing.")


# ---------------------------------------------------------------- E7: challenge drain
def e7_challenges():
    rows = []
    ritual = 0.005
    for cap_bps, acct_bps in ((100, 10000), (100, 500), (10, 500)):
        for k in (1, 5, 20):
            per_day = min(1.0, k * acct_bps / 10000) * cap_bps / 10000
            rows.append([f"{cap_bps / 100:.1f}% / {acct_bps / 100:.0f}%", k, f"{per_day * 100:.3f}%",
                         f"{(1 - (1 - per_day) ** 90) * 100:.1f}%", f"{per_day / ritual * 100:.0f}%"])
    table("E7 Challenge drain: the creator signs completions for wallets of their own",
          ["Daily cap / per-account cap", "Creator's undetected full-stake wallets", "Pool taken per day", "Pool taken in 90 days", "Relative to the ritual's daily payout"],
          rows,
          "Challenges are the one place the creator has discretion, so they are boxed in: a daily cap for all "
          "challenges together, a per-account share of that cap, and the same anti-farm weight as the ritual "
          "(each wallet needs a full, aged stake to collect its share). Production values: 0.1% / 5%, so the "
          "worst case is a fifth of what the ritual pays the community each day, and it takes 20 fully staked "
          "undetected wallets to reach it.")


def charts():
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        return
    # E2 as a chart: hit rate by attendance, two rules
    att = [1.0, 0.75, 0.5, 0.35, 0.25, 0.18, 0.12]
    old, new, xs = [], [], []
    for scale in att:
        w = build("organic", False, 7, attendance_scale=scale)
        hits = 0
        tot_p = sum(a.checkin_p for a in w.accounts) / len(w.accounts)
        for d in range(DAYS):
            r = step(w, d)
            indep = independent_units([a for a in w.accounts if a.active], d)
            hits += int(r["checkins"] >= max(P["quorum_min"], 0.5 * indep))
        w2, _ = run("organic", False, 7, attendance_scale=scale)
        xs.append(tot_p * 100)
        old.append(hits / DAYS * 100)
        new.append(w2.days_hit / DAYS * 100)
    fig, ax = plt.subplots(figsize=(8.6, 4.2))
    style(ax, "Days the community hits its daily target, by how many holders check in")
    ax.plot(xs, new, color=C["blue"], linewidth=2, marker="o", markersize=6, label="Retargeting turnout (contract)")
    ax.plot(xs, old, color=C["orange"], linewidth=2, marker="o", markersize=6, label="Fixed target: 50% of holders (first design)")
    ax.set_xlabel("average share of holders who check in on a given day (%)", color=C["muted"], fontsize=9)
    ax.set_ylabel("days hit, out of 60 (%)", color=C["muted"], fontsize=9)
    ax.set_ylim(0, 105)
    ax.legend(frameon=False, fontsize=9, loc="center right")
    fig.tight_layout()
    fig.savefig(os.path.join(OUT, "attendance.png"), dpi=130, facecolor="white")


PREAMBLE = """# INSYDR strength test

We attacked our own design before asking anyone else to trust it. This file lists what broke, what we
changed in the contract because of it, and what is still weak. Every number below is regenerated by
`python sim/strength_test.py`, and every contract rule named here is pinned by a unit test in
`contracts/insydr/src/tests.rs`.

## What broke, and what changed

| # | Attack or flaw we found | How bad | What the contract does now | Pinned by |
|---|---|---|---|---|
| 1 | **Dust aging.** Park 1 token in a wallet for weeks, then load it with a big stack: the wallet looked like an old holder with a big stake. | High: it defeated the holding-age ramp and the 14-day rule for counting holders. | Holding age is **coin-age**: an inflow dilutes age in proportion (`age' = age x old / new`). | `dust_aged_wallet_gains_nothing_when_loaded`, `rotating_one_stack_through_wallets_does_not_multiply_it`, `gifting_coins_cannot_lower_someone_elses_weight` |
| 2 | **Impossible daily target.** The target was "50% of holders check in every day". At realistic attendance it is missed every single day (E2). | High: the pool would burn daily and the streak would never exist. | The target **retargets like a difficulty**: 75% of the community's own trailing-average turnout. Turnout is measured in anti-farm weight, so dust wallets cannot move it. No oracle number is involved. | `the_daily_target_follows_the_communitys_own_turnout`, `dust_wallets_cannot_move_the_daily_target` |
| 3 | **The oracle was an admin key in disguise.** One key could report a million holders and mint the whole reserve in one transaction, with 20% going to the creator. | High: it contradicted "no admin". | **m-of-n** oracle signatures; **one tranche per cooldown**; the reported count **cannot exceed on-chain holders**; the creator's cut **vests** and is **clawed back** on contraction (E4, E5). | `one_oracle_alone_cannot_attest_when_two_are_required`, `the_same_oracle_cannot_sign_twice`, `oracle_cannot_report_more_holders_than_exist`, `a_compromised_oracle_cannot_mint_the_reserve_at_once`, `creator_cut_vests_linearly`, `contraction_claws_back_the_unvested_creator_cut` |
| 4 | **Challenge rewards ignored stake.** Any aged wallet got the full reward, so 100 dust wallets completing a creator's game earned 100 rewards. | High: the creator challenge budget was a free farm. | Challenges use the **same weight as the ritual** (linear in aged stake), plus a **per-account share cap** of the daily challenge budget (E7). | `challenge_rewards_scale_with_stake_so_splitting_is_neutral`, `one_account_cannot_take_the_whole_challenge_budget`, `challenges_cannot_drain_the_pool` |
| 5 | **Streaks paid scripts.** A streak that resets on a miss gives a cron job up to 1.5x on top of its attendance advantage (E1). | Medium. | A miss costs `streak_miss_penalty` days of streak instead of resetting it. | `streak_decays_instead_of_resetting` |
| 6 | **Clustering evidence lived off-chain.** The oracle's flags could only be checked against a JSON file we publish. | Medium: "trust us". | The contract records each holder's **majority token source** (the account most of its coins came from). Anyone can read it with `get_holders` and re-derive the clusters. | `majority_source_shows_where_a_wallets_coins_came_from` |
| 7 | **Two burn paths were missing from the ledger** (closing an account with a balance, refund to a closed account). | Low: the "ledger reconciles with supply" claim could have gone false. | Both are recorded. A randomized test runs 6,000 operations and checks five accounting invariants after each one. | `closing_an_account_with_a_balance_is_a_recorded_burn`, `fuzz_accounting_invariants_hold` |
| 8 | **Idle coins drained themselves.** A coin deployed before its community arrived burned its pool and lost every season (this happened to our first two testnet coins). | Medium. | The day clock starts at the first attestation that reports `quorum_min` independent holders. | `idle_coin_is_not_drained_before_the_ritual_starts`, `no_check_in_before_start` |
| 9 | **Anyone could trigger the creator's claim.** The coins still went to the creator, but the timing was not theirs. Found by the coin page's own `attack` command. | Low. | `claim_creator` is creator-only. | `a_stranger_cannot_trigger_the_creators_claim` |

## The experiments

"""

EPILOGUE = """
## What is still weak (not solved, stated plainly)

1. **A patient, well-funded farmer is a holder.** Someone who buys separately for each wallet, funds each
   from an unrelated source and keeps the coins for weeks is not detected. They earn what the same coins
   would earn in honest hands, no more (E6), and they evade the whale cap. We think that is the honest
   limit of any design without identity: at that point the farmer has paid the full economic cost of
   being N holders.
2. **Checking in is scriptable.** INSYDR does not prove a human is present. It makes automation worth
   only what attendance is worth (E1), and makes wallet count worth nothing.
3. **The oracle is still trusted for two things:** when supply tranches mint, and which wallets are
   flagged. It is bounded (E5) and its evidence is on-chain, but m-of-n signers are a committee, not a
   proof. A false flag reduces a wallet's rewards; it can never freeze or take coins.
4. **Majority source can be dodged** by feeding a wallet from three relays in equal parts. The oracle
   follows source chains to a root, which catches one hop of relays, not a deliberate mesh.
5. **Rewards are redistribution, not yield.** The pool is funded by supply that mints when the community
   grows. People who show up are paid by diluting people who do not. When growth stops, the pool decays
   by 0.5% a day and rewards shrink. That is intended, and it means INSYDR is a retention mechanic, not
   an income stream.
6. **A locked contract cannot be patched.** Deleting the keys is what makes the rules credible, and it
   also makes every bug permanent. Migration would mean a new coin and a wrapper.
7. **Storage is paid by the contract's NEAR balance.** Members, days, nonces and ledger entries grow with
   use. Anyone can top the account up; nothing in the contract does it automatically.
8. **Challenges are creator discretion.** The creator decides who "completed" a challenge. The caps bound
   the damage (E7); they do not make the creator honest.
9. **Not audited. Testnet only.**
"""

if __name__ == "__main__":
    e1_streak_fairness()
    e2_attendance()
    e3_exodus()
    e4_fake_milestone()
    e5_oracle()
    e6_sybil()
    e7_challenges()
    charts()
    with open(os.path.join(OUT, "report.md"), "w") as f:
        f.write("\n".join(REPORT))
    doc = os.path.join(HERE, "..", "docs", "STRENGTH-TEST.md")
    with open(doc, "w") as f:
        f.write(PREAMBLE + "\n".join(REPORT) + EPILOGUE)
    print("\nwrote", OUT, "and docs/STRENGTH-TEST.md")
