#!/usr/bin/env bash
# Run once on the machine that holds the live coin's keys (app/.keys). Packs them into the
# INSYDR_KEYS repository secret so .github/workflows/keeper.yml can keep the live coin's scripted
# community checking in every coin day, even when this machine is asleep.
#
# Testnet only: refuses if app/.keys holds anything other than testnet keys.
set -euo pipefail
cd "$(dirname "$0")/.."
REPO="${REPO:-hpark236/NEAR-hackathon-project}"
[ -f app/data/live.json ] || { echo "no app/data/live.json: this is not the live coin's machine"; exit 1; }
[ -d app/.keys/testnet ] || { echo "no app/.keys/testnet here"; exit 1; }
if ls app/.keys | grep -qv '^testnet$'; then echo "app/.keys holds more than testnet keys; refusing"; exit 1; fi
coin=$(node -p 'require("./app/data/live.json").coin')
creator=$(node -p 'require("./app/data/live.json").main')
[ -f "app/.keys/testnet/$creator.json" ] || { echo "the key for $creator is not here: wrong machine"; exit 1; }
echo "uploading $(ls app/.keys/testnet | wc -l | tr -d ' ') testnet keys for $coin as the INSYDR_KEYS secret of $REPO"
tar czf - -C app .keys | base64 | tr -d '\n' | gh secret set INSYDR_KEYS --repo "$REPO"
echo "done. Start a first run now: gh workflow run keeper.yml --repo $REPO"
