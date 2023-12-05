# Spin up fendermint local network
This will guide you through how to setup a 4-node fendermint network on local machine for debugging.

### Create validators
The validators are already created, their credential information are stored in `abci*`. If you want to use your own set
of validators, just regenerate the private key file on your own and replace the files.

### Fund validators
Fund your validators in subnet `/r314159`.

### Create subnet
Create subnet with the following command, make sure you have `ipc-cli` installed.
```shell
<PATH TO IPC>/ipc-cli subnet create --parent /r314159 --min-validator-stake 0.04 --min-cross-msg-fee 0 --min-validators 4 --bottomup-check-period 10
```
Note down the subnet id from the command output.

### Join subnet
Ask your validators join the subnet with:
```shell
SUBNET=<SUBNET ID FROM PREVIOUS COMMAND> IPC_PATH=<YOUR IPC CLI FOLDER>/ipc-cli bash ./join_subnet.sh
```
make sure you have `jq` installed.

### Generate genesis
Generate the genesis with:
```shell
$FENDERMINT_CLI genesis --genesis-file ./genesis.json ipc from-parent -s /r314159/${SUBNET} --parent-endpoint "https://api.calibration.node.glif.io/rpc/v1" --parent-gateway ${GATEWAY} --parent-registry ${REGISTRY}
```

### Generate keys
```shell
FENDERMINT_CLI=<PATH TO YOUR FENDERMINT CLI>
```