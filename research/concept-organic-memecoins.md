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
| **No mint authority** | Supply changes only through the coded rules below. Keys are deleted after deploy. | Rugs by minting |
| **Creator earn-out** | The creator allocation (e.g. 5%) unlocks in steps *only* when the coin reaches milestones of **independent** holders who've held ≥ N days (e.g. 100 / 500 / 2,000). | Dev dumps; pays for community, not hype |
| **Patience-weighted rewards** | A community pool pays out by **holding age × participation**. Someone who bought an hour ago gets ~nothing; a months-long holder who checks in gets the most. | Flippers, pump groups |
| **Insider decay** | Wallets flagged as linked to the deployer or launch snipers can't earn rewards, and their *creator-side* or early allocation vests slowly. They can still sell what they bought (no honeypot). | Insider clusters |
| **Rituals** | Daily check-in streak (gasless, can run from Telegram with a key that can only call `check_in`). The community hits the quorum → participants get a small reward; misses → a slice of the pool burns. | Turns "community" into something you do, not a slogan |
| **Referrals that can't be farmed** | A referrer is paid only when the referred account is **independent** (not in the same cluster) **and** holds ≥ N days. | Reply-guy shilling, sybil referral farms |
| **Official channel registry** | The contract stores the coin's official links and states that **the contract is the only distribution channel**. Any "drop your address for an airdrop" is fake by definition, and anyone can check that. | Fake-airdrop phishing |

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
