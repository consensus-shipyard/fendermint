# Tendermint

To implement the [architecture](./architecture.md) we intend to make use of the following open source components to integrate with Tendermint:

* [Tendermint Core](https://github.com/tendermint/tendermint): The generic blockchain SMR system. In particular we shall use the upcoming [v0.37](https://github.com/tendermint/tendermint/tree/v0.37.0-rc2) version which has the required [extensions](./architecture.md#abci) to [ABCI++](https://github.com/tendermint/tendermint/tree/v0.37.0-rc2/spec/abci). Note that the `tendermint/tendermint` repo is going to be archived; in the future it's possibly going to be developed further at https://github.com/informalsystems/tendermint, and further derivations will be registered at https://github.com/tendermint/ecosystem
* [tendermint-rs](https://github.com/informalsystems/tendermint-rs/) is a Rust library that contains Tendermint Core [datatypes](https://github.com/informalsystems/tendermint-rs/tree/main/tendermint); the [proto](https://github.com/informalsystems/tendermint-rs/tree/main/proto) code [generated](https://github.com/informalsystems/tendermint-rs/tree/main/tools/proto-compiler) from the Tendermint protobuf definitions; a synchronous [ABCI server](https://github.com/informalsystems/tendermint-rs/tree/main/abci) with a trait the application can implement, with a [KV-store example](https://github.com/informalsystems/tendermint-rs/blob/main/abci/src/application/kvstore/main.rs) familiar from the tutorial; and various other goodies for building docker images, integration testing the application with Tendermint, and so on. Lucky for us there is a [draft PR](https://github.com/informalsystems/tendermint-rs/pull/1193) to compile the protobuf definitions for both the current `v0.34` and the upcoming `v0.37` version, so we can even use that branch as a dependency to get the right data types and not have to do any proto compilation on our end!

Another project worth looking at is Penumbra's [tower-abci](https://github.com/penumbra-zone/tower-abci) which adapts the ABCI interfaces from `tendermint-rs` to be used with [tower](https://crates.io/crates/tower) and has a [server](https://github.com/penumbra-zone/tower-abci/blob/main/src/server.rs) implementation that works with [tokio](https://crates.io/crates/tokio). So, unlike the ABCI server in `tendermint-rs`, this is asynchronous; even if we don't use it, it's easy to follow as an example.

That should be enough to get us started with Tendermint.

## Install Tendermint Core

TODO: Instructions on downloading the right version of tendermint, building and installing.

## Smoke Test with the `kvstore`

TODO: Do the simplest check to see if the tendermint-rs ABCI example works on the PR branch that targets 0.37
