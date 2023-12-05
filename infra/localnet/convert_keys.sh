#!/usr/bin/env bash

${FENDERMINT_CLI}/fendermint key from-eth -s ./abci0/private_key -n abci0 -o ./abci0
${FENDERMINT_CLI}/fendermint key from-eth -s ./abci1/private_key -n abci1 -o ./abci1
${FENDERMINT_CLI}/fendermint key from-eth -s ./abci2/private_key -n abci2 -o ./abci2
${FENDERMINT_CLI}/fendermint key from-eth -s ./abci3/private_key -n abci3 -o ./abci3

${FENDERMINT_CLI}/fendermint key into-tendermint --secret-key ./abci0/abci0.sk --out ./abci0/validator.json
${FENDERMINT_CLI}/fendermint key into-tendermint --secret-key ./abci1/abci1.sk --out ./abci1/validator.json
${FENDERMINT_CLI}/fendermint key into-tendermint --secret-key ./abci2/abci2.sk --out ./abci2/validator.json
${FENDERMINT_CLI}/fendermint key into-tendermint --secret-key ./abci3/abci3.sk --out ./abci3/validator.json

chmod 600 ./abci0/validator.json
chmod 600 ./abci1/validator.json
chmod 600 ./abci2/validator.json
chmod 600 ./abci3/validator.json

rm ./abci0/abci0.*
rm ./abci1/abci1.*
rm ./abci2/abci2.*
rm ./abci3/abci3.*