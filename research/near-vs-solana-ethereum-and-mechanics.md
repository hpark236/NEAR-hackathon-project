# NEAR vs. Solana/Ethereum, meme.cooking vs. pump.fun, and what "mechanics" look like

Written 2026-11-09. Sources: earlier research files in this folder, the meme.cooking API, and the track prompt. Figures from the hackathon thesis entries are cited as such.

## 1. NEAR compared with Solana and Ethereum

| | **NEAR** | **Solana** | **Ethereum (L1)** |
|---|---|---|---|
| Fees | Fractions of a cent | Fractions of a cent (plus priority fees when busy) | Dollars or more on L1; cheap on L2s |
| Speed | ~1 s blocks, ~2 s finality | ~0.4 s slots, fast confirmation | ~12 s blocks, ~13 min full finality |
| Token model | **Every token is its own contract** (NEP-141), fully programmable | Almost all tokens share **one program** (SPL Token / Token-2022); custom behavior limited to built-in extensions (transfer fee, transfer hook) | Every token is its own contract (ERC-20), fully programmable |
| Accounts | **Readable names** (`alice.near`), sub-accounts | Random base58 addresses | `0x…` addresses (ENS optional) |
| Key permissions | **Native scoped keys:** a key that can only call `check_in()` on one contract, with a gas allowance | One key controls everything | One key controls everything (smart wallets / ERC-4337 add this, not native) |
| Gasless UX | **Native meta-transactions** (NEP-366): a relayer pays gas | Fee payer can be another account | Needs ERC-4337 paymasters |
| Cross-chain | **Chain Signatures + NEAR Intents built in:** pay from BTC, ETH, SOL or USDC on Base | Bridges | Bridges / L2s |
| Composability | Cross-contract calls are **asynchronous** (not atomic) | Atomic within one transaction | Atomic within one transaction |
| Memecoin market | Small. On-chain activity is down sharply from its peak (thesis entries report about -84% in transactions) | **Dominant** memecoin chain: deepest liquidity, bots, tools | Large but expensive; most memes moved to L2s or Solana |
| Dev revenue | **The called contract earns a share of gas fees** | No | No |

**Short version:**
- **vs. Solana:** NEAR gives you a fully programmable token plus scoped keys and native gasless and cross-chain features. Solana gives you the users and liquidity.
- **vs. Ethereum:** NEAR gives you the same programmability at near-zero cost with built-in UX features Ethereum needs add-ons for.

## 2. meme.cooking vs. pump.fun

| | **pump.fun (Solana)** | **meme.cooking (NEAR)** |
|---|---|---|
| Price discovery | **Bonding curve:** price rises with each buy, trading is continuous | **Fair auction:** everyone deposits in a time window (5 min to 1 day) and **everyone pays the same price** |
| Sniping | Severe. The first buyers get the cheapest price, so bots and insiders buy in the first blocks | Largely removed inside the auction, since there is no "earlier = cheaper" |
| Graduation | Once the curve fills, liquidity migrates to a DEX (PumpSwap) | If the soft cap is met (about 100 NEAR on recent launches), the token deploys, the LP goes to Rhea and **the LP is locked** |
| Failure | The token exists either way and usually dies on the curve | No token is created if the cap isn't met (**2,052 of 2,319** launches never launched) |
| Token | Standard fixed-supply SPL token | Standard fixed-supply NEP-141 (`name-N.meme-cooking.near`) |
| Custom mechanics | **None** | **None** |
| Scale | Millions of launches | ~2.3k launches, 267 live tokens |

**Takeaway:** both are *launch* machines. Neither gives a coin any behavior after launch. A mechanic layer that plugs into either is new.

## 3. NEAR's weaknesses we can address, and strengths we can use

### Weaknesses → what we do about them
| Weakness | How the project addresses it |
|---|---|
| **Few users, thin meme liquidity** | Let people join **from any chain via NEAR Intents** (buy or check in with SOL, ETH or USDC). The mechanic doesn't need NEAR natives. Note: Intents is mainnet-only. |
| **Wallet friction** (new users must get NEAR, install a wallet) | **Gasless meta-transactions** plus **Telegram or email login** (HOT wallet `.tg` accounts already appear among meme.cooking creators). Rituals happen from a chat with no wallet pop-ups. |
| **Storage registration** (a wallet must register before it can hold a token) | The contract **auto-registers** participants on first action and pays storage from a small fee or the treasury. |
| **Async cross-contract calls** (not atomic) | Keep the mechanic **inside one contract** (token + rules together). Where we must call out (e.g. reading a Rhea price), write explicit callbacks and fail closed. |
| **Most meme.cooking tokens die** (11.5% launch, and many live ones go quiet) | Make the mechanic **attachable to existing NEAR memes**, giving dead communities a reason to show up daily. |
| **Sybil farming** (accounts are cheap) | Weight participation by holding or time held, charge a small fee, or require a stake. Document the limits plainly, as the prompt asks. |

### Strengths → what we lean on
1. **The token is our own program.** Mint and burn rules live *in the coin*, not in a backend. This is the core of the track and the main advantage over pump.fun/SPL tokens.
2. **Scoped keys.** Give a Telegram bot a key that can **only** call `check_in()` for that user. Daily rituals become one tap, and the bot can't move funds.
3. **Native gasless.** Users never need NEAR to participate.
4. **Cross-chain entry via Intents.** "Join the streak with SOL" turns NEAR's liquidity weakness into a reach story.
5. **Named accounts.** Leaderboards show `hugo.near`, not `7xKp…`, which makes on-chain "high scores" feel social.
6. **Locking the contract** (delete the keys). A provable "no admin, rules are final."

## 4. What the three mechanic categories look like

Already taken by other entries (avoid copying):
- GRIP (lock-to-mint/burn)
- Calydon (commit together or burn)
- $BEFEWER (minority game)
- OATH (enforceable creator promises)
- Tripwire (rug-proof launchpad)
- $GRIND (agent staking/slashing)
- Swarmpad (proof-of-work unlocks)

### A. Supply that reacts to the community
- **Streak mint / burn:** if enough holders check in today, check-ins earn a small mint; if not, a slice of the treasury burns and the streak resets. (The organizers' example, unbuilt.)
- **Whale damper:** a transfer that would push any wallet above X% of supply is taxed, and the tax burns or goes to the treasury. Concentration becomes expensive.
- **Insider decay:** wallets flagged as launch insiders or snipers (by on-chain clustering) have balances that vest slowly or decay into the treasury. (Builds on `ihaterugs` work.)
- **Breadth mint:** new supply unlocks only when the number of *independent* active holders crosses milestones. Rewards real growth, not volume.

### B. Treasuries with rules
- **Floor defender:** fees fill a NEAR treasury, and when the price falls X% below its 7-day average on Rhea, the contract buys back and burns. It needs a price read and a callback, and it must be manipulation-aware.
- **Dead-man's treasury:** if the community goes silent for N days, the treasury pays out to the last N active participants. It's a reward for showing up when nobody else does.
- **Challenge bounties:** the treasury pays whoever completes an on-chain-verifiable task (e.g. bring 50 new holders through Intents this week).
- **No discretionary spending:** the treasury has no owner key. Only the coded rules can move funds.

### C. On-chain rituals and challenges
- **Daily check-in** with a public streak counter and a "longest streak" high score.
- **Weekly sacrifice:** holders burn tokens into the week's fire. The biggest offering gets an on-chain title or badge, and part of the pot goes back to everyone who participated.
- **Hot potato:** a cursed bag of tokens must be passed to a *new* holder within 24 h or it burns. It forces spread, and NEAR's named accounts make the chain of passes a readable story.
- **Seasons:** each 30-day season ends with a ritual settlement. Streak champions are recorded on-chain and parameters for the next season come from a pre-committed formula, not a vote by insiders.

## 5. A coherent design (proposal to discuss)

**One coin, three layers, all in one locked contract:**
1. **Ritual:** a daily check-in (gasless, from Telegram via a scoped key) drives a community streak.
2. **Reactive supply:** hitting the quorum mints to participants and missing it burns from the treasury. Insider-flagged wallets can't earn mints and decay instead.
3. **Rule-bound treasury:** funded by a small transfer fee; it only burns, rewards streaks, or pays the dead-man's payout. No owner.

**Selling points for judges:** the mechanic is in the token, there's no admin key, it's gasless and cross-chain, sybil limits are stated honestly, and there's a 60-day supply simulation, which the prompt explicitly asks for.
