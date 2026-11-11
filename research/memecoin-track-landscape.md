# Memecoins With Mechanics: the field on 11 November 2026

Source: `https://nearhackathon.com/api/projects` (494 projects, 28 in the memecoins track; it was 18 of 336 two days earlier), the public text of each demo post, and the repositories we could find. Re-run `research/scripts/fetch_projects.sh` to refresh.

This note exists to keep INSYDR honest about where it stands. It describes other teams' work as their own posts and READMEs describe it.

## What judges look at

From the hackathon site and `HACKATHON.md`:

- Claude Code and Codex screen every project first: does it run, and does it do what the README says. Humans decide.
- Previous NEAR hackathons scored technical implementation, design, impact, originality and sustainability.
- A hosted demo is a plus. A local-only demo needs a reproducible README and a video.
- The track is "judged on the mechanic, not the chart".

## The entries closest to INSYDR

| Project | Mechanic (as described by the team) | What it does well | Sybil / insider handling |
|---|---|---|---|
| Four Horsemen of the Memepocalypse | Daily check-in; turnout decides mint or treasury burn; three misses in a row burn 5% | Strong meme identity and art, a sandbox test of 283 signed transactions, clear README | A refundable 100-token bond per account. The README says funded holders can split across accounts. Not deployed publicly. |
| MIRA | Quests with staked entries and sealed votes; an activity "mood" sets how hard failed stakes burn; idle holders are taxed when the coin starves | The most thorough README in the track (a "what is real and what is not" table, a judging-criteria table), CI, live app, locked config, 0 keys | Stakes make sybil voting cost something. The demo faucet gives every account free coins, which the README states. |
| $YAP | Mints when the community talks on near.social, burns when it goes quiet; scored by a DAO | Uses near.social and a DAO, zero admin keys | Not described in the post |
| GRIP | Supply mints when 60% of the float is locked and burns under 35% | One clear rule, no owner functions | Not described |
| Calydon | Commit before a deadline or 10% of each stake burns | One contract, 0 access keys, hosted status page | Not described |
| STAG | Supply follows a repeated stag hunt | Game-theory framing, hosted page | Mentioned as a limit |
| LIFO | In a crash the reserve pays the newest buyers first | Original idea, 36 tests, CI, hosted app | Mentioned once |
| Tripwire | Launchpad where each token account has zero keys and rug attempts fail | The "rules, not promises" pitch, live testnet demo | Not the focus |
| OATH / Memeproof | Coins that register enforceable promises; a watcher triggers enforcement | Registry and dashboard | Not the focus |
| Swarmpad | Supply unlocks only when work is proven | Agent angle, 37 tests, hosted | Not the focus |
| $BEFEWER | Coin mined by winning a minority game three times | Hosted game, 89 tests | Not the focus |
| Petri Dish | Coins fed to cells in an on-chain Game of Life | Playable without a wallet | Not the focus |

Others in the track (Crownfall, BONFIRE, Realsies, $HACK, $GRIND, PAC, DORKS, Greed Machine, Meridian, Sidelot, Kimaex, TheRascals, NDBOT, BAG HEIST, Petri) lean on games, agents or trading tools more than on the token mechanic.

## What the strongest entries have in common

1. A hosted page a judge can open.
2. Tests in the tens (30 to 90), often with a sandbox or integration run.
3. CI.
4. A locked contract with 0 access keys.
5. A simulation, and limits stated in the README.
6. Something a judge can do without a wallet.

## Where INSYDR stands against that list

| | Status on 11 November |
|---|---|
| Hosted page | **Missing.** The page is a static file ready for GitHub Pages; it needs the repository to be public. |
| Tests | 44 contract tests (one is a 6,000-operation fuzz), 13 oracle tests, 24 end-to-end checks on a local sandbox |
| CI | Written (`.github/workflows/ci.yml`), **not pushed**: the GitHub token lacks the `workflow` scope |
| Locked contract | Yes: `idog-bqdss4.testnet`, 0 keys from the first minute, code hash matches the committed wasm |
| Simulation and limits | Yes, plus a strength test of our own design (`docs/STRENGTH-TEST.md`) |
| Wallet-free trial | Yes: guest account, tap contract and an "attack it" command in the page's terminal |
| Meme identity | **Weak.** INSYDR has a tagline but no art or lore. Four Horsemen and Petri Dish are far ahead here. |

## What INSYDR does that the others do not

- It treats the wallet farm as the main problem. In every other streak or turnout coin we read, a check-in or a vote is counted per account, so the mechanic pays whoever makes the most accounts. Their READMEs mostly say so.
- Coin-age instead of account age, and an on-chain record of where each wallet's coins came from.
- A daily target that retargets to the community's own turnout instead of a number fixed at launch.
- A creator allocation that depends on growth, vests, and is clawed back if the growth was fake.
- A bounded oracle (m-of-n, rate-limited, sanity-checked against on-chain holders).

## What we should still learn from them

- **Four Horsemen:** a memecoin needs a meme. Named outcomes and art make a mechanic memorable.
- **MIRA:** seeded "Judges: do this" states on the live coin so a visitor has something to do at any hour.
- **Petri Dish, $BEFEWER:** "free, no wallet" in the first line of the post.
