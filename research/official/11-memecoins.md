# 11. Memecoins With Mechanics

**Track id:** `memecoins` · **Tagline:** The joke needs rules

Build memecoins where the meme does something: supply that reacts to the community, treasuries with rules, onchain rituals and challenges. Judges score the mechanic, and the price chart doesn't count.

**Lean on:** Memecoins, token mechanics, NEP-141

## Start here

- [NEP-141 fungible token standard](https://github.com/near/NEPs/blob/master/neps/nep-0141.md)
- [meme.cooking](https://meme.cooking/), the memecoin launchpad partner
- [near-smart-contracts agent skill](https://github.com/near/agent-skills/tree/main/skills/near-smart-contracts)

## Prompt

```text
Track: Memecoins With Mechanics (id: memecoins).

Build [a streak coin]. Every day holders can check in once. If the community hits the daily check-in target, every holder who checked in gets a small mint. If the community misses it, a slice of the treasury burns and the streak resets. The longest streak is printed on the token page like a high score.

Requirements:
- An NEP-141 token with the mechanic in the contract, not in a backend somebody could switch off.
- All parameters (target, mint size, burn size) set at launch and readable by anyone. No admin key that can change them later.
- Sybil resistance good enough for a demo, with its limits written plainly in the README.
- A page with the current streak, today's progress and a log of every mint and burn.

Simulate 60 days of community behavior first and show me supply over time under a few scenarios. Then we pick the parameters.
```
