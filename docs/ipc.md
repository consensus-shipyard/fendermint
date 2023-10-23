# IPC

This documentation will guide you through the different utils provided in Fendermint for the deployment of Fendermint-based IPC subnets. This docs are only focused on the infrastructure deployment, for an end-to-end walk through of spawning IPC subnets refer to the [IPC quickstart](https://github.com/consensus-shipyard/ipc/blob/main/docs/quickstart-calibration.md).

## Deploy subnet bootstrap
In order not to expose directly the network address information from validators, subnets leverage the use of bootstraps (or `seeds` in CometBFT parlance), for new nodes to discover peers in the network and connect to the subnet's validators. To run a bootstrap node you can run the following command from the root of the repo:
```bash
cargo make --makefile infra/Makefile.toml bootstrap
```
You'll see that by the end of the output, this command should output the network address of your bootstrap. You can use this endpoint to include this bootstrap node as a seed in the `seeds` configuration of CometBFT.
```console
[cargo-make] INFO - Running Task: cometbft-wait
[cargo-make] INFO - Running Task: cometbft-node-id
2b23b8298dff7711819172252f9df3c84531b1d9@172.26.0.2:26650
[cargo-make] INFO - Build Done in 13.38 seconds.
```

If at any time you need to query the endpoint of your bootstrap, you can run: 
```bash
cargo make --makefile infra/Makefile.toml bootstrap-id
```

`cargo-make bootstrap` supports the following environment variables to customize the deployment:
- `CMT_HOST_PORT` (optional): Specifies the listening port in the localhost for COMETBFT.

Finally, to remove the bootstrap you can run:
```
cargo make --makefile infra/Makefile.toml bootstrap-down
```


## Deploy child subnet validator
Once a child subnet has been bootstrapped in its parent, its subnet actor has been deployed, and has fulfilled its minimum requirements in terms of validators and minimum collateral, validators in the subnet can deploy their infrastructure to spawn the child subnet.

In order to spawn a validator node in a child subnet, you need to run:
```bash
cargo make --makefile infra/Makefile.toml -e VALIDATOR_PRIV_KEY=<VALIDDATOR_PRIV_KEY> -e CHAIN_NAME=<SUBNET_ID> -e CMT_HOST_PORT=<COMETBFT_PORT> -e COMMA_SEPARATED_BOOTSTRAPS=<BOOTSTRAP_NODE1>,<BOOTSTRAP_NODE2> -e ETHAPI_HOST_PORT=<ETH_RPC_PORT> child-validator
```
This command will run the infrastructure for a Fendermint validator in the child subnet. It will generate the genesis of the subnet from the information in its parent, and will run the validator's infrastructure with the specific configuration passed in the command.

`cargo-make child-validator` supports the following environment variables to customize the deployment:
- `CMT_HOST_PORT` (optional): Specifies the listening port in the localhost for COMETBFT.
- `ETHAPI_HOST_PORT` (optional): Specifies the listening port in the localhost for the ETH RPC of the node.
- `NODE_NAME` (optional): Name for the node deployment. Along with `CMT_HOST_PORT` and `ETHAPI_HOST_PORT`, these variables come really handy for the deployment of several validator nodes over the same system.
- `VALIDATOR_PRIV_KEY`: Path of the private key for your validator (it should be the corresponding one used to join the subnet in the parent).
- `CHAIN_NAME`: SubnetID for the child subnet.
- `COMMA_SEPARATED_BOOTSTRAPS`: Comma separated list of bootstraps (or seeds in CometBFT parlance).
- `PARENT_ENDPOINT`: Public endpoint that the validator should use to connect to the parent.
- `PARENT_REGISTRY`: Ethereum address of the IPC registry contract in the parent
- `PARENT_GATEWAY`: Ethereum address of the IPC gateway contract in the parent.

Finally, to remove the bootstrap you can run:
```
cargo make --makefile infra/Makefile.toml child-validator-down
```
