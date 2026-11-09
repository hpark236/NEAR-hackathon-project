# Grassroots: the platform (current direction)

Decided 2026-11-09. This supersedes the single-coin framing in `concept-organic-memecoins.md` (its mechanics carry over) and drops `concept-proof-of-practice.md`.

## One line

**Grassroots is infrastructure that gives a memecoin a reason to exist.** We build the sticky parts: streaks, incentives, a rule-bound treasury and anti-farming. Any creator plugs in **their own** challenge, game or community activity, and the coin rewards real participation instead of insiders and shillers.

## The pitch: "back to the good old days"

**Then.** Memecoins started as community. In 2013–2014, Dogecoin was a tipping currency on Reddit and Twitter. Its community crowdfunded the Jamaican bobsled team's trip to the Sochi Olympics and sponsored a NASCAR car. The joke *was* the community, and the coin was how it did things together.

**Now.** Launchpads mint thousands of coins a day. Most are designed to be dumped on the next buyer: insiders and snipers in the first block, "drop ur address for 100 [coin]" engagement farming, Telegram pump groups, rugs.
- Even big names are caught up in it. In October 2026, a token promoted from trader Kimchi's account right after he signed a Pump.fun partnership hit about $40M in market cap within an hour, then fell 98% after the post vanished. Whether it was a hack or a stunt is still disputed, and buyers lost either way. [Protos](https://protos.com/pump-funs-kimchi-partnership-has-turned-into-chaos/), [Bitcoin.com News](https://news.bitcoin.com/market-updates/kimchi-x-account-hacked-project-kimchi-token-crash/), [Cryptopolitan](https://www.cryptopolitan.com/kimchi-token-sheds-98-30-minute-run-15m/)
- *Pitch wording rule:* say "promoted from his account" and "hack or stunt, still disputed." Do **not** say he rugged anyone; that is unproven.

**Grassroots.** Coins whose supply, treasury and rewards follow a *real*, *active* community, enforced by a locked NEAR contract nobody can change. The only way to earn is to show up, not to be early or loud.

## Architecture

```
            ┌──────────────────────── Grassroots core (one locked contract per coin) ───────────────────────┐
creator ──▶ │ NEP-141 token │ growth tranches │ community treasury │ streak engine │ anti-farm multipliers │
            │                                ▲ payouts only via ▼                                          │
            │                       Challenge registry (budgets, caps, cooldowns)                          │
            └───────────────▲──────────────────────▲───────────────────────────▲───────────────────────────┘
                            │                      │                           │
              built-in challenges       issuer-signed challenges      contract-verified challenges
              (check-in streak,         (creator's game/server/bot    (any NEAR contract implementing
               bonfire, hold-the-line,   signs "X completed Y";        verify(account, proof) → bool;
               real-friend referral)     core checks ed25519 sig)      core calls it, pays in callback)
```

### What the core guarantees (creators can't change these)
- **No discretionary mint.** Supply comes only from growth tranches (unlocked by independent-holder milestones) and capped challenge rewards.
- **Bounded challenge budgets.** Each challenge gets a fixed per-epoch budget from the challenge pool, plus per-account daily caps. A buggy or dishonest challenge can't drain the treasury.
- **Anti-farm on every payout.** Whatever the challenge, the core multiplies each reward by:
  - an **independence factor** (from the Organic Score: wallets clustered together share one reward);
  - **holding age**;
  - **streak length**.

  Creators can't accidentally create something farmable.
- **Never blocks selling.** No transfer tax, no sell locks (those are honeypot traits and break Rhea pools).
- **Public log.** Every mint, burn and payout is an on-chain event shown on the coin page.

### What creators control
- The coin's name, art and links (stored on-chain as the *only* official channels).
- Which built-in challenges are on, and their reward weights within limits.
- Their own challenges: anything from a browser game to a Telegram activity, community art contests (judged by their server) or IRL meetups (QR check-in signed by their bot).

### Three ways a challenge proves completion
1. **Built-in:** the core enforces it by itself (daily check-in, hold-the-line, bonfire, referrals).
2. **Issuer-signed:** the creator's server signs a claim, and the core verifies the signature with `env::ed25519_verify`. You trust the creator, but only up to that challenge's capped budget.
3. **Contract-verified:** the creator deploys a NEAR contract with a `verify` method. The core calls it asynchronously and pays in the callback (no trust in the creator's server).

### Players
- Sign in with email or Telegram, with no wallet pop-ups or gas (NEP-366 meta-transactions, keys scoped to `check_in` / `claim`).
- One page per coin shows: streak, season goal, your rewards, Organic Score, and the supply-vs-real-community chart.

## Exactly what NEAR does (and what is off-chain)

**The test:** if Grassroots ran on a normal database, whoever owns the database (us, or the creator) could quietly mint, change the rules or drain the pool. That is the rug we're fighting. NEAR is where the rules live so that **nobody, including us, can change them**.

| Piece | NEAR feature | Why it matters |
|---|---|---|
| The coin | A **NEP-141 contract** per coin, deployed by our factory as a sub-account (`dogrun.grassroots.near`) | Each coin's rules live in its own code (not possible with standard SPL tokens on Solana) |
| "Nobody can change it" | **Delete the full-access keys** after deploy, and the code is frozen | The provable no-rug guarantee; anyone can check the account has no keys |
| Treasury | Funds held **by the contract itself**, moved only by its code | No owner wallet to drain |
| Creator challenges (signed) | **`env::ed25519_verify`** inside the contract | Verifies a creator's signed "X completed Y" on-chain, capped by budget |
| Creator challenges (contract) | **Cross-contract calls + callbacks** (promises) | The core asks the creator's verifier contract and pays only if it says yes |
| Daily check-in with no friction | **Function-call access keys** scoped to `check_in` / `claim`, plus **NEP-366 meta-transactions** (a relayer pays gas) | Players never buy NEAR or approve pop-ups, which is critical for non-crypto users |
| Identity | **Named accounts** (`hugo.near`) and implicit accounts for email/Telegram sign-ups | Readable leaderboards and readable forensics evidence |
| Public history | **NEP-297 events** emitted for every mint, burn and payout | Anyone (or any indexer) can audit the full history |
| Joining from other chains | **NEAR Intents** (mainnet only) | Buy in with SOL, ETH or USDC, offsetting NEAR's small user base |
| Launch & liquidity (optional) | **meme.cooking** fair auction, **Rhea** pool | Partner integrations; no sniper curve |
| Trustworthy scoring (stretch) | **NEAR AI Cloud TEE** | Organic Score computed in attested hardware, not "trust our server" |

**Off-chain (stated plainly in the README):**
- The Organic Score indexer. It publishes signed results; the contract only trusts its signature, within limits.
- Creators' own game or app servers.
- The web page and relayer. Neither can move funds; they only submit what users sign or what the contract allows.

## Positioning: niche or broad?

**Memecoins are supposed to be niche.** Dogecoin was a niche joke with a tight community. The answer isn't to make one coin for everyone; it's **thousands of small niche communities, each with its own coin**, all on the same guardrails. Breadth comes from the *number* of communities, not from turning memecoins into a loyalty program.

**Decision for the hackathon:**
- **Pitch and build = memecoin communities.** That's the track, the judges' criteria and the "back to the roots" story.
- **Non-crypto groups (run clubs, language apps, hobby groups) = one "where this goes" line** in the pitch: "the same engine works for any community that wants to reward showing up." Our streak and anti-farm core doesn't care what the challenge is.
- **Avoid tuition centers and anything aimed at minors.** A tradeable token for kids raises gambling and financial-regulation questions and would hurt the pitch.
- Why not lead with the broad version: run clubs and apps mostly want **points, not a tradeable coin**. Once a reward trades, speculation and farming follow. That tension is exactly what the memecoin framing owns, while the broad framing would have to explain it away.

## Demo plan (we are the first creator)

A testnet coin with:
- the **built-in daily streak** plus a **season challenge**;
- **one tiny creator challenge** to prove the plug-in model (an issuer-signed mini-game or Telegram activity, to be decided);
- **a farming attack in the demo:** 50 sybil accounts check in daily and earn about one account's worth, because the core collapses their cluster. This is the moment that sells the anti-farm claim.

## Tracks
- `memecoins` (primary): mechanics.
- `social`: community-owned treasury, earned rewards.
- `security`: insider and farm defense.
- `games`: only if the demo challenge is a game.

## Next steps
1. A 60-day simulation (organic / insider pump / sybil farm) to fix the parameters.
2. The Rust core contract: token, tranches, treasury, streak, challenge registry with three verifier types, anti-farm multipliers. Tests.
3. A lightweight Organic Score (funding-source and timing clustering) that publishes signed attestations.
4. The coin page plus the demo challenge.
5. README, demo video, X post draft.
