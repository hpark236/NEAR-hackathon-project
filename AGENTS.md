# Notes for coding agents

INSYDR is a NEP-141 streak memecoin on NEAR testnet. The rules live in one Rust contract (`contracts/insydr/src/lib.rs`); `app/` holds the oracle, the demo scripts and the end-to-end test; `web/index.html` is the single-file coin page; `sim/` holds the simulations.

To verify the project, follow [JUDGE.md](JUDGE.md). In short:

```bash
cd contracts/insydr && cargo test                       # 44 unit tests (Rust stable, about 2 min on a first build)
cd ../../app && npm ci && npm test && npm run e2e       # 13 oracle tests, then 24 checks on the real wasm in a local NEAR sandbox (3 to 4 min, Node 22)
cd .. && python3 sim/insydr_sim.py && python3 sim/strength_test.py && git status --short sim docs   # regenerates the published results; tree stays clean
```

- The e2e test needs no Rust and no network beyond downloading the sandbox binary once. On macOS the sandbox needs `xz` (`brew install xz`).
- `npm run e2e` rewrites `app/data/e2e-report.json`; that is its report, not a failure.
- The live page can be checked headlessly: open `https://hpark236.github.io/NEAR-hackathon-project/?run=all` and read `window.INSYDR_CHECKS` once all seven checks are in it (`locked`, `accounting`, `split`, `farms`, `ratio`, `checkin`, `attacks`). This signs testnet transactions from a throwaway guest account; no wallet or funds are needed.
- The live contract is `idog-bqdss4.testnet`. It has 0 access keys and its code hash is `A2K4Vj76jmrqGBG5FRHVsnB6CRgauYAZYWBtFkciXNNZ`, which is base58(sha256(`contracts/insydr/res/insydr.wasm`)).
- `node demo.js run` replays the whole scenario on public testnet with fresh faucet accounts (a new coin, real time, rate-limited faucet); `npm run e2e` is the same scenario in a few minutes and is the better check. `node demo.js keep --state live` acts on the live coin and needs keys that are not in the repo.
- Known limits are listed at the end of [JUDGE.md](JUDGE.md) and in the README's Limits section.
