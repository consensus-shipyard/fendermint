#!/usr/bin/env bash

$IPC_PATH subnet join \
  --from "0x$(cat ./abci0/wallet.json | jq -r '.address')" \
  --subnet /r314159/${SUBNET} \
  --collateral 0.01 \
  --public-key "$(cat ./abci0/wallet.json | jq -r '.pubkey')"

$IPC_PATH subnet join \
  --from "0x$(cat ./abci1/wallet.json | jq -r '.address')" \
  --subnet /r314159/${SUBNET} \
  --collateral 0.01 \
  --public-key "$(cat ./abci1/wallet.json | jq -r '.pubkey')"

$IPC_PATH subnet join \
  --from "0x$(cat ./abci2/wallet.json | jq -r '.address')" \
  --subnet /r314159/${SUBNET} \
  --collateral 0.01 \
  --public-key "$(cat ./abci2/wallet.json | jq -r '.pubkey')"

$IPC_PATH subnet join \
  --from "0x$(cat ./abci3/wallet.json | jq -r '.address')" \
  --subnet /r314159/${SUBNET} \
  --collateral 0.01 \
  --public-key "$(cat ./abci3/wallet.json | jq -r '.pubkey')"