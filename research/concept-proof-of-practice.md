# Concept extension: proof-of-practice rewards (streaks, levels, learning)

Drafted 2026-11-09. Builds on `concept-organic-memecoins.md` (Grassroots).

## The idea

Coins are earned by **showing up and getting better**, not by being early or shilling:
- **Duolingo-style:** practice daily, keep a streak, and the streak earns coins.
- **Games:** a small game dev rewards real players at "Level 10" or after 7 consecutive days played.
- **Learning:** solve math problems, finish lessons, learn a skill.

It's a playful echo of Bitcoin: there, computers race to solve a puzzle to unlock new coins. Here, **people unlock new coins by doing the practice.**

## Competition check (336 submissions, 2026-11-09)

Nothing mentions Duolingo, quizzes, lessons, achievements or play-to-earn. The nearest entries:
- **Grind District:** a learning game with a testnet "Skill Passport" (credentials, no coin rewards).
- **Swarmpad:** token supply unlocks via proof of work (markdown "mining levels").
- **Pull Up:** event attendance streak with deposits.

**The lane is open.**

## Why this fits Grassroots

Grassroots asks how a coin can reward only a real community. Proof-of-practice gives the clearest answer: **new supply is created only when real people do real practice.**
- Streak = the daily ritual
- Levels / lessons = challenges
- Supply grows with verified practice = supply that reacts to the community
- The reward pool pays by formula = treasury with rules

## The hard problem: who proves the practice happened?

A contract can't see your screen. There are three ways to prove it, each with a different trust level:

| Proof | How it works | Trust | Weakness |
|---|---|---|---|
| **Issuer-signed** | The game or app server signs "player `hugo.near` reached Level 10 on day X". The contract verifies the signature (NEAR has a built-in ed25519 check, `env::ed25519_verify`) and pays. | Trust the dev. That's fine, since it's **their** coin and their reward budget. | The dev could sign fake claims for themselves, so cap issuer-claimable supply per epoch and make every claim public. |
| **Contract-checked puzzles** | The contract generates a per-user problem from a random seed (math, logic), the user submits an answer, and the contract checks it. | Trustless | **Bots and AI solve these faster than humans.** It works as a fun ritual, not as proof of a human. (The same reason CAPTCHAs have failed.) |
| **Proof of time** | Check in on N distinct days. The contract enforces one per day. | Trustless | Time can't be faked, but **accounts can be multiplied.** |

**Plain truth:** nobody can prove *trustlessly* that a human learned something. What we can build:
1. Let the **issuer** define what counts (signed claims).
2. Use **time** (streaks) that can't be rushed.
3. Use Grassroots' **anti-farm rules** so running 100 bot accounts doesn't pay: patience-weighting, cluster detection, per-account caps, and rewards that ramp up with streak length.

## Lessons from play-to-earn (Axie Infinity, etc.)

Play-to-earn economies died when rewards were large enough to attract farmers, who sold everything and crashed the token. Design rules that follow from that:
- **Rewards are small and capped per account per day.** Fun first, coins second (the games track says "judged on the game loop, not the token").
- **Emissions follow real growth.** A fixed schedule doesn't. (This is the Grassroots growth tranches.)
- **Streak multipliers** reward consistency over volume, so long-held streaks beat grinding many accounts.
- **There must be something to spend it on** besides selling: cosmetics, entry to tournaments, tipping, the bonfire ritual.

## Product shape: "Grassroots Rewards" kit

For small creators (indie game devs, teachers, study groups, habit apps):
1. **Create a coin** from the Grassroots template: name, supply rules, reward table ("7-day streak = 50", "Level 10 = 200"), daily caps.
2. **Plug in proofs:** a small SDK that signs achievement claims from their server, and/or built-in contract-checked puzzles and streaks.
3. **Players claim gaslessly:** sign in with email or Telegram, with no wallet pop-ups (meta-transactions plus a key scoped to `claim`/`check_in`).
4. **Public page:** the coin's streak leaderboard, supply-vs-real-players chart and Organic Score.

**Demo for judges (first creator = us):** a tiny daily practice game in the browser, e.g. 5 quick math or logic questions a day with streaks and levels (it fits the Bitcoin "solve to unlock" metaphor). Players earn the coin. The supply chart shows it growing only with real daily players.

**Tracks:** `memecoins` (mechanics) + `games` (real economy, chain invisible to players), maybe `social`.

## Five-day scope impact

This adds a demo app (the practice game) and issuer-signed claims to the Grassroots plan. To stay on schedule, the **Organic Score** becomes a lighter version: basic clustering (shared funding source, creation timing) instead of the full Solana forensics port.
