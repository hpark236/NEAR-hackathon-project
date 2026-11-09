# Primer: NEAR, smart contracts, and memecoins

Plain-language background for the project, written 2026-11-09. For build details see `memecoins-near-integration.md`.

## 1. What NEAR Protocol is

NEAR is a **layer-1 blockchain**, a base network like Ethereum or Solana rather than something built on top of one. It launched its mainnet in 2020, co-founded by Illia Polosukhin (a co-author of the Transformer paper, "Attention Is All You Need") and Alexander Skidanov.

**What it is designed for:**
- **Proof of stake:** validators lock (stake) NEAR tokens to produce blocks and earn rewards.
- **Sharding ("Nightshade"):** the chain is split into parallel shards, so it can process more transactions as it adds shards. Users and apps don't see the split.
- **Fast and cheap:** blocks about every second, finality within a couple of seconds, fees usually a fraction of a cent.
- **Human-readable accounts:** addresses look like `alice.near`, not `0x3f2a…`. Accounts can have sub-accounts (`treasury.alice.near`).
- **Keys with permissions:** an account can hold several keys. A *full-access* key can do anything. A *function-call* key can only call specific functions on one contract, with a limited gas allowance. This is why so many hackathon projects use it for "agent can only call `pay()`."

**The NEAR token** pays for gas, pays for on-chain storage (you lock NEAR in proportion to the data you store), and is what validators stake. Most gas fees are burned, and a share goes to the contract that was called.

**Current focus (relevant to the hackathon):**
- **NEAR Intents:** you say what you want ("swap my USDC on Base into SOL"), and market makers compete to fill it across chains. Per this hackathon's thesis entries, it moved roughly $30B+ over the past year.
- **Chain Signatures:** a NEAR account can sign transactions on Bitcoin, Ethereum and other chains.
- **NEAR AI:** private AI inference in secure hardware (TEEs).

## 2. What a "NEAR smart contract" is

A **smart contract is a program that lives on the blockchain.** Its code and data are stored on-chain. Anyone can call its functions by sending a transaction, and every validator runs the same code and must agree on the result. Nobody, including its author, can quietly change what it did.

**On NEAR specifically:**
- **One account, one contract:** a contract is deployed *to an account*. `streak.alice.near` is both an address and a program.
- **Written in Rust (or JavaScript)** with `near-sdk`, compiled to **WebAssembly**, then uploaded.
- **State is key-value storage** inside that account. The account must lock about 1 NEAR per 100 KB stored, which is why contracts usually make users pay for their own storage (`storage_deposit`).
- **View vs. change methods:** view methods (e.g. "what's my balance?") are free reads. Change methods (e.g. "transfer") are signed transactions that cost a little gas.
- **Asynchronous cross-contract calls:** when contract A calls contract B, B runs in a later block and A gets the result in a callback. This differs from Ethereum, where the whole call chain happens at once. You must handle "B failed" explicitly.
- **Immutability is your choice:** if the account still has a full-access key, the owner can redeploy new code. **Delete all full-access keys** and the contract is permanently locked. That's how you prove "no admin can change the rules."

**Analogy:** a NEAR contract is like a vending machine bolted to a public street. The rules are printed on the front, anyone can use it, and once you throw away the key, nobody can open it to change the rules.

## 3. How memecoins work in general

A **memecoin** is a token whose value comes almost entirely from attention, community and narrative, not from cash flows or utility. Examples include DOGE, and on Solana, thousands of pump.fun launches.

**Typical lifecycle:**
1. **Creation:** someone deploys a token with a name, ticker, image and fixed supply, often 1 billion.
2. **Launch / price discovery:** usually one of these:
   - **Bonding curve** (e.g. pump.fun): price rises automatically as people buy. When a threshold is reached, liquidity "graduates" to a DEX.
   - **Fair auction / presale:** everyone deposits during a window and gets the same price.
   - **Direct pool:** the creator pairs tokens with SOL, ETH or NEAR in a DEX pool.
3. **Trading on an AMM:** a liquidity pool (token + base asset) sets the price by formula (x·y=k). Buying pushes the price up, selling pushes it down.
4. **Attention cycle:** price follows social momentum. Most memecoins go to near zero; a few run.

**Where the risk comes from** (the same patterns `ihaterugs` traced on Solana):
- **Insiders and snipers:** the creator or bots buy in the first blocks, often from many wallets, then sell into later buyers.
- **Rug pulls:** the team pulls liquidity, or keeps the power to mint more tokens.
- **Honeypots:** contracts that let you buy but not sell.
- **Concentrated supply:** a few wallets hold most of the float.

**What this track wants instead:** a coin whose **rules do something**. Supply reacts to community behavior, the treasury follows fixed rules, and rituals or challenges reward participation. All of it is enforced by code, so it isn't "trust the dev."

## 4. Memecoins on NEAR specifically

- **Every token is its own contract.** A NEAR token is a contract implementing **NEP-141**, NEAR's ERC-20 equivalent: `ft_transfer`, `ft_balance_of`, `ft_total_supply`, etc.
- **Key contrast with Solana:**
  - On Solana, almost all tokens share one program (SPL Token), so custom behavior is limited to the extensions that program offers.
  - **On NEAR, each token's contract is yours to write**, so the mechanic can live *inside* the token: mint on streaks, burn on failure, tax insider wallets.
  - That's the main reason this track exists on NEAR.
- **Sending tokens to a contract:** `ft_transfer_call` sends tokens to a contract *and* notifies it (`ft_on_transfer`), so a contract can react to deposits. It's NEAR's version of "approve + call" in one step.
- **Storage registration:** a wallet must be registered on a token (a small NEAR storage deposit) before it can hold that token.
- **Ecosystem:**
  - **meme.cooking** is the main launchpad and a hackathon partner. It runs fair auctions in wNEAR; if the minimum is reached, it creates the token as `name-N.meme-cooking.near` and locks liquidity on **Rhea** (formerly Ref Finance, NEAR's main DEX). As of today, 267 of 2,319 launches graduated.
  - Other launch venues show up in hackathon entries (e.g. JustHoot, nearly.trade).
  - Known NEAR memes include Blackdragon, Shitzu (the team behind meme.cooking), LONK and NEKO.
- **The limit that matters for us:** meme.cooking always creates a plain fixed-supply token. A custom mechanic must either be your own token contract or a separate contract that holds and manages a treasury of a meme.cooking token. See `memecoins-near-integration.md`.
