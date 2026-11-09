# NEAR HACKATHON: landscape research

Snapshot taken **2026-11-09**, 5 days before the deadline (**2026-11-14 23:59 UTC**).

## Method and coverage

- **Projects:** all **336** public submissions, from the site's own directory API (`https://nearhackathon.com/api/projects`).
- **Demo posts:** read the X post text for **312/336** through X's public oEmbed endpoint. The other 24 were deleted, private or failed to load.
- **Repos:** **90** posts link a GitHub repo directly. Another ~113 were matched by repo name from a GitHub search. Those matches are marked *unverified* in `data/projects.csv`, because common names like "Leash" or "Aegis" can collide. I read README excerpts for 203 repos.
- **Track prompts:** I read the organizers' example prompt for every track (`nearai/near-hackathon-prompt-examples`).
- **Not done yet:** I did not watch demo videos or read X replies. Claude in Chrome wasn't enabled in this session.
- **Raw data:** `data/projects.csv` has one row per project, with tracks, demo link, repo, repo description and full post text.

## Headline findings

1. **Every track pays the same prizes:** $10k / $3k / $2k for 1st / 2nd / 3rd, plus one $55k grand prize. Crowding varies a lot, though. Projects per track (most enter several):

   | Track | Entries | | Track | Entries |
   |---|---|---|---|---|
   | security | 165 | | thesis | 30 |
   | agents | 163 | | swarms | 29 |
   | realworld | 69 | | games | 27 |
   | business | 58 | | nfts | 23 |
   | defi | 57 | | **social** | **22** |
   | open | 56 | | **memecoins** | **18** |
   | privacy | 54 | | | |

2. **About half of all entries are the same product:** an "AI agent spending firewall" (a vault contract, a function-call key that can only call `pay()`, caps, an allowlist, human approval, "prompt injection blocked"). I counted 100+ variants, many with the same names: 6× Leash, 5× AgentGuard, 3× Allowance, Tollgate ×2, Veto, Bridle, Capgrate, SpendGuard, and more. **Don't enter security/agents with a spend guard.**

3. **Many teams built the organizers' example prompt verbatim.** Five of the thirteen example ideas are now crowded:
   - The thesis example ("does Intents volume lead price?"): ~25 projects.
   - The realworld example (invoice links): ~10.
   - The games example (market town with NPC merchants): ~9.
   - The nfts example (NFT that owns a sub-account): ~9.
   - The open example (inheritance / dead-man switch): ~5.

   **Three example ideas were never built:** the **DeFi cross-chain DCA vault**, the **memecoin "streak coin"**, and (mostly) the **social bounty board**. Patch Fund comes closest to the bounty board.

4. **Most partners have almost no integrations:** **GoMining 0, meme.cooking 0, BRRRAIN 0, NEAR Business 0, HOT 0, near.social 0, Trezu 2, Aurora 2, Agent Market 2-3, Rhea ~5.** Judges include partners, so a real partner integration stands out.

## Track by track

### `defi`: Unstoppable DeFi (57)
**Aims at:** exchanges, perps, lending, vaults and trading agents that settle cross-chain through NEAR Intents and Chain Signatures.
**What exists:**
- **Agent-trading guards:** nearswaps agent kit, OneShot (no double-fills), Hallmark (worst-price bound), Lifeguard (repay-only Aave rescuer), Charter/Aureus (TEE-guarded AI funds).
- **Intents tooling:** Kalfou Swap (verify the router), Quittance (cross-chain receipts), Vault Check (reserve alarm), Intent Sentinel, Wakeless (TWAP-sliced confidential swaps), Terra matcher (order matching), Unpair (Zcash unlinking).
- **Markets:** setllend (fixed-rate lending + order book), Warrant (oracle-free options), OneUp (perps where losers get a "Life NFT"), Logline (prediction markets on on-chain metrics), Dragons Market / Priced In / Credence (prediction markets), BEXLEY (no-liquidation leverage), OmniYield, NEAR Private OTC.
- **Terminals and copilots:** ASGARD, Allpilot, NDBOT.

**Open gaps:**
- **Cross-chain DCA vault** (the organizers' example; nobody built it).
- Rhea-native vaults and CLMM tooling.
- Perps or basis trades that settle through Intents.
- Liquidation protection for NEAR-native lending (Lifeguard only covers Aave on Base).
- Delta-neutral "earn" strategies.

### `thesis`: Investment Thesis (30)
**Aims at:** a re-runnable, evidence-based long-term NEAR thesis.
**What exists:** ~25 near-identical dashboards on "Intents volume vs. value accrual / issuance coverage", plus NEAR Books (TEE-attested revenue) and nearbi/Tune Up (paid research agents).
**Verdict:** **Saturated.** Every entry reaches the same conclusion ("usage up, value capture small").

### `privacy`: Privacy & Private AI (54)
**Aims at:** TEEs, NEAR AI Cloud and confidential finance.
**What exists:**
- **Private documents:** Sealed Ledger (the organizer example), Sealed Dossier (legal), RedactGate (PII firewall), Áurea (health/FHIR), Aftercall (meeting notes).
- **Proofs:** Jurat (balance threshold), VeilProof (agent solvency), TEE Receipts, Measure (TDX attestation).
- **Markets and finance:** Undiluted (confidential cap table), DeadDrop (sell encrypted files), Redline / Sealed Exploit / Skarbnik (sealed bug bounties), PredicTEE, NOVA (confidential copy-trading), Realsies (human-or-AI game).

**Open gaps:**
- Private credit scoring or underwriting (prove income or repayment history without revealing it).
- Confidential payroll or salary benchmarking.
- Private order flow or RFQ for real businesses.
- TEE-sealed sports/trading signals sold with a provable track record.

### `business`: Autonomous Business (58)
**Aims at:** AI-run businesses with treasury, sales, procurement and revenue share, under human-set constraints.
**What exists:** mostly spend guards relabeled as business. Real businesses include Staffless (translation shop), AllTrades (local SMB marketing), PinPay (pay-per-click agents), Throughput Treasury, Outlive/Deed (sell a running business as an NFT), Tijar, ProofAgent.
**Open gaps:**
- A business that actually earns revenue from outside customers, end to end, with public books. Most entries are demos with fake customers.
- An agent listed on **Agent Market** and earning there.

### `swarms`: AI Swarms (29)
**Aims at:** multi-agent markets, bidding, group buying and emergent economies.
**What exists:** C0-op and Flock (group buying, the organizer example), HiveBid (swarm auctions), SwarmShop (bug-fix bidding), Swarm-for-Hire, CartelBreaker (anti-collusion auctions), Loca (agent-to-agent credit), Tontine (market-maker firms that go bankrupt), Agent Undercover (social deduction).
**Open gaps:**
- Agent **prediction or forecasting swarms** that settle on NEAR.
- Agent **insurance pools / mutuals**.
- Agent **labor unions or price-setting cooperatives**.
- **Reputation markets** where agents stake on each other.

### `security`: AI Security & Defense (165)
**Aims at:** spending limits, policy engines, sandboxed wallets, reputation and scam detection.
**What exists:** 100+ spend firewalls, plus counterparty checkers (NearGuard, Tripwire, Gideon, Safe Near, Mint Scout, Glassbox, KeyProof) and a few originals: Writ / Vouch (who pays when the AI is wrong: bonds and guarantors), Clean Exit (revoke leftover agent keys), Crucible (con-the-guard game), CartelBreaker, Black box (flight recorder).
**Verdict:** **Avoid.** Placing here would need something much better than 165 lookalikes.

### `agents`: Agent Economy (163)
**Aims at:** agents that earn, spend, negotiate, hire other agents and pay for APIs, compute or data.
**What exists:**
- **Pay-per-call / x402:** Tollgate (payment channels), Penstock (x402 paywall), PayPerCall (escrowed calls), x402-near, Saku.
- **Hiring:** Relay and Foreman (Agent Market), Bonded (performance bonds), Judge on NEAR, NEARPORTAL (bounties for merged PRs).
- Most of the rest are spend guards.

**Open gaps:**
- An agent that **sells real data or compute** on Agent Market and shows earnings.
- **Agent subscriptions** (recurring billing between agents).
- **Agent tax and accounting export.**

### `realworld`: Real-World Money (69)
**Aims at:** payroll, invoicing, remittances, merchant payments and stablecoin rails for normal companies.
**What exists:**
- **Invoice / pay links (crowded):** Cobra (voice-to-invoice), Invoice Link, Payday, Settl, Settleform, Paylink, Pelargo.
- **Escrow:** MilestonePay, SpecLock, ProofPay, Payproof, ClearSend.
- **Everyday payments:** NearPocket (Asian QR rails), NairaRails (Nigerian bills), CareRail (clinic bills), Dues (remittance checkout), AJO (savings circle), Roomy (split expenses), Bolsillos (family envelopes), Setoff (inter-firm debt netting), Presafe (ticket refunds), NearHr (payroll).

**Open gaps:**
- **Recurring merchant subscriptions** paid from any chain.
- **Refunds and chargebacks** for crypto merchants.
- **Tax / 1099 / accounting export** for stablecoin businesses.
- **Supplier early payment** (invoice factoring). Veridom Pay touches this.
- **Merchant loyalty / cashback.**

### `games`: Games With Real Economies (27)
**Aims at:** NPCs with wallets and player-owned economies. Judged on the game loop, not the token.
**What exists:**
- **Market towns (crowded):** Nims, Meridian, Lantern Caravan, Bazaar Sovereign, DeepSea Hunter, DiveMarket, Worldo, Nearverse, Q-day.
- **Others:** Overclock (AI agent duels), Agent Undercover, Realsies, DUEL (friendly bets), Crucible, TokenGo, $BEFEWER (minority game), Afterlight.

**Open gaps:**
- **Strategy or card games where AI opponents bankroll themselves.**
- **Skill games with provable fair play.**
- **Tournament circuits with agent sponsors.**
- Anything that is not a market town.

### `nfts`: NFTs That Do Something (23)
**Aims at:** NFTs as accounts that hold money, earn royalties and carry access rights.
**What exists:** almost all "the NFT owns a keyless account": Deed, Outlive, Handover, Logen, Torch, ArchivePass, Release Passport, Phylaxtag (warranty fund), Agent NFT, Veridom (receivable NFT), OpenNear (bridge to OpenSea).
**Open gaps:**
- **Credentials and access NFTs that gate real things.** Tickets that refund themselves, licenses that expire.
- **Rental / lending of NFT-accounts.**
- **NFTs whose behavior changes with on-chain events**, i.e. dynamic NFTs that actually react.

### `memecoins`: Memecoins With Mechanics (18): least crowded
**Aims at:** memes with real mechanics (reactive supply, rule-bound treasuries, rituals). Judged on the mechanic, not the chart.
**What exists:**
- **Supply and participation mechanics:** GRIP (supply reacts to locked float), OATH (enforceable creator promises), Calydon (commit together or burn), $BEFEWER (minority-game mining), Swarmpad (proof-of-work unlocks), $GRIND (agent staking/slashing).
- **Launchpads and treasury plays:** Tripwire (rug-proof launchpad), PAC, Greed Machine, $HACK, Realsies.

**Open gaps:**
- The **streak coin** (the organizer example, unbuilt).
- **Any meme.cooking or BRRRAIN integration** (0 so far).
- **Insider-proof launches:** on-chain clustering penalizes or vests sniper/insider wallets.
- **Community-vs-whale mechanics.**
- **Prediction-meme hybrids.**

### `social`: Social Money (22)
**Aims at:** shared budgets, programmable rewards, collective ownership and reputation-gated access on Sputnik DAO and Trezu.
**What exists:** Patch Fund (the bounty board example), Petty Cash (budget rules), FairShare (peer credit allocation), Headcount (sybil-screened payouts), AllTogether and Lockstep (all-or-nothing funding), Daykeeper (one-day power auctions), Termwork (DAO hires an agent), Pull Up, Potluck, Chai Pot.
**Open gaps:**
- **Quadratic or retroactive funding with sybil resistance** (0 entries).
- **near.social integration** (0).
- **Trezu-native apps** (only 2).
- **Membership dues and access tiers.**
- **Reputation that travels across DAOs.**

### `open`: Open Innovation (56)
**Aims at:** anything that doesn't fit a track.
**What exists:**
- **Inheritance (crowded):** Heir, Pal, Morrowseal, Torch.
- **Infrastructure:** PROVEN (verifying RPC), Tidepool (Intents testnet sandbox), Shelflife (source-link check), NEARpedia (MCP for agents).
- Plus many cross-listed projects.

## Recommended ideas (feasible solo in 5 days)

Ranked on four things: open lane, partner fit, judge appeal, and fit with skills already shown in your other repos (memecoin forensics, LMSR prediction markets, cross-venue arbitrage, poker/game-theory bots, portfolio stress testing).

1. **Insider-proof memecoin launch (`memecoins` + `security`).**
   - **Mechanic:** a NEP-141 launch where wallets clustered as insiders or snipers (funded from the deployer, buying in the first blocks, sharing funding sources) get their allocation vested or taxed by the contract. Holders who pass the check get a rebate.
   - **Why it fits:** reuses your Solana forensics work (`ihaterugs`, `memecoin-wallet-tracker`).
   - **Partner hook:** launch on **meme.cooking** (0 integrations so far).
   - **Field:** the smallest track. Tripwire and OATH are adjacent but don't do wallet clustering.

2. **Streak coin with community stakes (`memecoins`).**
   - **Mechanic:** the organizers' own example, which nobody built. Daily check-ins; hit the target and checked-in holders get a mint, miss it and part of the treasury burns.
   - **Scope:** the smallest build of these five. Pairs with idea 1 as a single mechanics-focused memecoin.

3. **Cross-chain DCA vault with slippage guard (`defi` + `realworld`).**
   - **Mechanic:** the organizers' DeFi example, also unbuilt. Deposit USDC from any chain, schedule buys of BTC/ETH/NEAR through 1Click, reject bad quotes, and handle REFUNDED/FAILED states.
   - **Extras:** add your portfolio stress-test angle, e.g. "what this schedule would have done in past crashes".
   - **Caveat:** Intents is mainnet-only, so it needs tiny amounts or `dry: true` quotes.

4. **Agent forecasting market (`swarms` + `agents`, maybe `defi`).**
   - **Mechanic:** reuse the LMSR engine from `consensus-engine`. AI agents trade on questions, the market settles on NEAR, and agents are scored against Polymarket/Kalshi.
   - **Field:** Priced In / Logline / Dragons exist, but none are agent-swarm forecasting with LMSR and an external benchmark.

5. **Quadratic/retro funding for Sputnik DAO with sybil scoring (`social`).**
   - **Mechanic:** the first QF/retro entry. Reuse wallet-clustering to discount sybil contributors. Headcount is the only neighbor, and it only screens payouts.

**Strategic note:** one project can enter several tracks, and some pairings are cheap to cover. For example, idea 1 entered in `memecoins` + `security` competes for first place in a track of 18 while staying eligible in security.

## Caveats
- The directory keeps growing until Nov 14. Re-run the scrape before committing to an idea.
- Repo matches marked *unverified* came from name search and may point to the wrong repo.
- I judged projects from post text and README excerpts, not by running them. Some entries are much stronger or weaker than their post suggests.
