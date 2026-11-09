# Concept: memecoins that can only grow organically (working name: **Grassroots**)

Drafted 2026-11-09. Status: proposal, not yet built.

## Thesis

People are tired of:
- **Rugs:** the dev dumps, pulls liquidity or mints more.
- **Insiders and snipers:** a cluster of wallets linked to the deployer buys first and sells into everyone else.
- **Engagement farming:** "GOING TO THE MOOOON, drop ur address and I'll send 100 [coin], join my Telegram." These posts exist because shilling pays: the shiller holds early bags, or the "airdrop" is a phishing funnel.

We can't stop people posting on X. **We can make the payoff go away.** If a coin's contract only rewards *real, independent, patient* community members, and makes insider bags and pump-and-dumps unprofitable, the shill has nothing to sell. Memecoins go back to being about the community, which is the point of the meme.

## Design principles

1. **Rules live in the token contract**, and the contract is locked (no admin keys). Nobody can change them after launch.
2. **Reward patience and participation; never block selling.** Blocking or heavily taxing sells is how *honeypots* work, so the design must never look like one. Exits are always free. Rewards go only to those who stay and show up.
3. **The creator gets paid for building a community, not for a pump.** The creator's allocation unlocks only as independent holders grow (an "earn-out").
4. **Every claim is checkable.** Insider flags come with evidence, scores are re-computable, and the only official distribution channel is the contract itself.
5. **State the limits plainly.** Sybil detection is heuristic; we say what it catches and what it doesn't.

## What we build (two parts)

### Part 1: the Grassroots token template (a NEAR contract)

A NEP-141 token factory. Each coin created from it gets these rules, fixed at launch:

| Rule | What it does | Problem it fights |
|---|---|---|
| **No *discretionary* mint** | Nobody can mint at will. Supply changes **only** through the coded rules below (organic-growth tranches, streak rewards, burns). Keys are deleted after deploy. | Rugs by minting |
| **Creator earn-out** | The creator allocation (e.g. 5%) unlocks in steps *only* when the coin reaches milestones of **independent** holders who've held ≥ N days (e.g. 100 / 500 / 2,000). | Dev dumps; pays for community, not hype |
| **Patience-weighted rewards** | A community pool pays out by **holding age × participation**. Someone who bought an hour ago gets ~nothing; a months-long holder who checks in gets the most. | Flippers, pump groups |
| **Insider decay** | Wallets flagged as linked to the deployer or launch snipers can't earn rewards, and their *creator-side* or early allocation vests slowly. They can still sell what they bought (no honeypot). | Insider clusters |
| **Rituals** | Daily check-in streak (gasless, can run from Telegram with a key that can only call `check_in`). The community hits the quorum → participants get a small reward; misses → a slice of the pool burns. | Turns "community" into something you do, not a slogan |
| **Referrals that can't be farmed** | A referrer is paid only when the referred account is **independent** (not in the same cluster) **and** holds ≥ N days. | Reply-guy shilling, sybil referral farms |
| **Official channel registry** | The contract stores the coin's official links and states that **the contract is the only distribution channel**. Any "drop your address for an airdrop" is fake by definition, and anyone can check that. | Fake-airdrop phishing |

### How Part 1 maps to the track's four phrases

The track asks for "supply that reacts to the community, treasuries with rules, onchain rituals and challenges." The unifying idea: **the coin's supply and treasury follow the size and loyalty of its *real* community.**

**1. Supply that reacts to the community**
- **Growth tranches:** most of the supply (e.g. 60%) isn't minted at launch. A tranche mints each time the coin crosses an **independent-holder milestone**. Supply literally grows with the real community, and fake holders don't count.
- **Who gets each tranche:** split between the community pool (to holders who stayed), the creator earn-out, and new-holder welcome grants.
- **Contraction:** if independent holders *fall* below a milestone, or a season challenge fails, a slice of the unminted reserve is permanently cancelled. The maximum possible supply shrinks.
- **Streak mints:** small daily mints to participants when the check-in quorum is hit (the organizers' streak-coin example).
- **Result:** the chart of total supply over time doubles as a chart of the community's real growth.

**2. Treasury with rules (the community pool)**
- **Funded by:** its share of each growth tranche, plus anything forfeited (insider rewards, the creator earn-out if milestones are never hit, unclaimed rewards).
- **No owner and no votes by insiders.** It can only do the following, by formula:
  - pay patience-weighted rewards each epoch;
  - pay streak rewards;
  - burn on a missed quorum or failed challenge;
  - make the dead-man payout: if the whole community goes silent for N days, the pool pays the last people still showing up.
- Everything it can ever do is readable in the contract.

**3. On-chain rituals**
- **Daily check-in (the heartbeat):** one tap, gasless, from the web or Telegram. It builds personal and community streaks, and the longest community streak is shown like a high score.
- **Weekly bonfire:** holders may burn tokens into the week's fire. The biggest offerings earn an on-chain title, and a share of the pool's weekly reward goes back to everyone who took part. It's a voluntary deflation ritual that signals commitment rather than speculation.

**4. Challenges (community-wide, per 30-day season)**
- **Season goal:** e.g. "+150 independent holders who each hold ≥ 14 days." Hit it → a bonus tranche mints to participants. Miss it → that tranche is cancelled for good.
- **Hold the line:** accounts whose balance didn't drop during the season get a bonus multiplier at season end. It's measured from on-chain balances alone, with no price oracle.
- **Bring a real friend:** referral rewards count only for independent referrals who stay 14+ days. They're paid at season end, so spam referrals never pay.

### Part 2: the Organic Score (public trust page + on-chain attestations)

A forensics indexer that works for **any** NEAR token, including existing meme.cooking coins:
- **Cluster-adjusted holders:** how many truly independent holders there are, after grouping wallets by shared funding source, timing and transfer patterns.
- **Insider share:** % of supply held by deployer-linked or first-block clusters.
- **Dev behavior:** has the creator sold, moved liquidity or funded new wallets?
- **Holder age distribution:** organic communities have long tails; pumps don't.
- **Wash trading:** volume cycling between linked wallets.

The indexer publishes **signed attestations on NEAR** (cluster flags, independent-holder counts). The Grassroots contract reads them for the earn-out, insider decay and referrals. Every flag links to its evidence (the transactions), so anyone can challenge it.

**Trust limit and mitigations:** the attestation signer is a trusted party. Mitigations:
- Publish all evidence.
- Add a challenge window before flags take effect.
- *(Stretch)* Run the scorer inside a NEAR AI Cloud TEE so its output is hardware-attested.

## Why NEAR (and not Solana/pump.fun)

- **Token rules can live in the token.** Each NEP-141 token is its own contract. On Solana, standard SPL tokens can't carry this logic (Token-2022 transfer hooks are limited), which is why pump.fun coins are all the same.
- **Scoped keys + gasless meta-transactions:** daily rituals from Telegram with a key that *can't* move funds.
- **Named accounts:** clusters and leaderboards are legible (`hugo.near`), which makes forensics evidence easier to read.
- **Intents:** people can join from SOL, ETH or USDC without learning NEAR, which helps with NEAR's thin user base.
- **meme.cooking's fair auction** already removes the bonding-curve sniper race. We cover what happens *after* launch, which nobody does.

## Known limits (to state in the README)

- **Clustering misses some cases.** An insider funding wallets from different CEX withdrawals is hard to link. We raise the cost of insider play; we don't make it impossible.
- **Sybil holders can still wait out the holding period.** Holding-age weighting makes this slow and capital-heavy, not impossible.
- **Fee-on-transfer breaks AMMs.** The design deliberately avoids transfer taxes so the token works normally in Rhea pools.
- **Existing meme.cooking tokens** can get the Organic Score and a rule-bound reward pool, but not the mint/earn-out rules, because their supply is fixed.

## Five-day MVP scope

1. **Day 1:** 60-day community simulation (organic vs. insider pump vs. sybil farm) to set parameters, per the track prompt.
2. **Days 1–2:** Rust contract covering NEP-141, earn-out, patience-weighted pool, check-in streak, attestation intake and links registry. Unit tests.
3. **Days 2–3:** Organic Score indexer for NEAR (port the clustering heuristics from the Solana forensics work), signed attestations to testnet.
4. **Day 4:** web page showing the coin's streak, Organic Score, earn-out progress and a mint/burn log. Telegram check-in bot (scoped key, gasless) if time allows.
5. **Day 5:** a testnet demo coin, README written for AI and human judges, a demo video, and an X post draft.

Tracks to enter: `memecoins` (primary), `security` (scam and insider defense), and possibly `social`.
