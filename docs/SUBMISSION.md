# Submission kit

Everything needed to submit, short of the things only the team lead can provide or do. Nothing here is sent anywhere by a script.

## Checklist (from the hackathon's `SUBMITTING.md`)

- [ ] Repository is public (`gh repo edit hpark236/NEAR-hackathon-project --visibility public`), and GitHub Pages serves `web/` so the coin page has a link
- [ ] CI is pushed and green (needs `gh auth refresh -h github.com -s workflow`, then commit `.github/workflows/ci.yml`)
- [ ] Demo post is live on X: says it was built at NEAR HACKATHON, tags @NEARProtocol and #NEARHackathon, has the video attached
- [ ] A mainnet NEAR account for the payout exists (the team's own; never invented)
- [ ] Every fitting track selected: `memecoins`, `social`, `security`
- [x] No keys or secrets in the repository history (scanned; keys live in `app/.keys/`, which is git-ignored)
- [ ] Submitted once, before 14 November 2026 23:59 UTC, after reading the exact payload

## Values only the team lead can give

Project name (suggested: `INSYDR`), team lead's name, contact email, team size, payout NEAR account, the demo post link, and consent to the contact notice.

## "What did you build?" (20 to 2,000 characters; this draft is about 1,700)

> Memecoins are farmed: insiders buy first, one person runs fifty wallets for every giveaway, and the creator can change the rules. The track's own example, a daily check-in streak coin, pays per wallet, so it pays whoever makes the most wallets. In our 60-day simulation a farmer with 13% of the coins takes 64% of the rewards with 300 wallets and 95% with 3,000.
>
> INSYDR is that streak coin with the farm, the insiders and the admin key taken out. It is one NEP-141 contract. Holders check in once a day; if turnout holds, the pool pays that day's check-ins, and if it drops, part of the pool burns and the streak resets. The target retargets to the community's own turnout, like a difficulty. Weight is linear in aged stake, so splitting a stack earns nothing, and age follows the coins, so pre-aged empty wallets gain nothing. New supply mints only when independent holders grow. The creator's cut vests and is clawed back if the growth was fake.
>
> NEAR is what makes this possible: every token is its own contract, so the rules live in the coin; the account's access keys are deleted, so nobody can change them; ed25519 signatures from a 2-of-3 oracle and from creators' challenge servers are verified on-chain; and the contract keeps its own holder list, coin-age, token-source record and ledger, so anyone can audit a farm flag over public RPC without an indexer.
>
> Live and locked on testnet (idog-bqdss4.testnet). 44 contract tests including a 6,000-operation fuzz, 24 end-to-end checks on a local sandbox with no wallet, and a page whose terminal lets you verify the claims and attack the contract yourself. We also publish what broke when we attacked our own design.

## Demo post (draft; the team lead posts it)

> Every memecoin giveaway is won by the guy with 300 wallets.
>
> INSYDR is a streak coin where 300 wallets earn what 1 does, the creator's own wallets earn 0.07%, and nobody can change a rule: the contract has zero keys.
>
> Try to break it in your browser, no wallet: [page link]
> Code: [repo link]
>
> Built at NEAR HACKATHON @NEARProtocol #NEARHackathon
>
> What would you attack first?

Optional tags for reach: @NearLegion @NearBuilders.

## Video (60 to 90 seconds; one complete moment, no slides)

1. **0:00** The coin page. Say the problem in one sentence: "The guy with 300 wallets wins every memecoin giveaway."
2. **0:08** Click **Join as guest**. A testnet account appears and gets 50,000 coins. No wallet.
3. **0:20** Click **Check in**. Point at the turnout meter and the line "fresh coins weigh almost nothing".
4. **0:30** Click **Farms**. Show the farm wallets and the on-chain evidence: "coins from farmer…".
5. **0:40** Point at **Who gets paid**: real holders against farm wallets against creator wallets.
6. **0:50** Click **Attack it**. Seven transactions, seven refusals from the contract.
7. **1:05** Click **Verify**: "0 access keys", "no mint, setter or upgrade among 47 public methods".
8. **1:15** One line on NEAR: "The rules live in the coin, and the keys are deleted. That only works because on NEAR every token is its own contract."

Alternative for a faster-moving recording: run `npm run e2e` in a terminal next to the page; ten days go by in two minutes, including the day the community stays away and the pool burns.
