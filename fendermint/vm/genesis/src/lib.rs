// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A Genesis data structure similar to [genesis.Template](https://github.com/filecoin-project/lotus/blob/v1.20.4/genesis/types.go)
//! in Lotus, which is used to [initialize](https://github.com/filecoin-project/lotus/blob/v1.20.4/chain/gen/genesis/genesis.go) the state tree.

use fvm_shared::{address::Address, econ::TokenAmount};

pub enum ActorMeta {
    Account {
        owner: Address,
    },
    MultiSig {
        signers: Vec<Address>,
        threshold: usize,
        vesting_duration: u64,
        vesting_start: u64,
    },
}

pub struct Actor {
    pub meta: ActorMeta,
    pub balance: TokenAmount,
}

/// Total stake delegated to this validator.
pub struct Power(u64);

/// A genesis validator with their initial power.
///
/// An [`Address`] would be enough to validate signatures, however
/// we will always need the public key to return updates in the
/// power distribution to Tendermint; it is easiest to ask for
/// the full public key.
///
/// Note that we could get the validators from `InitChain` through
/// the ABCI, but then we'd have to handle the case of a key we
/// don't know how to turn into an [`Address`]. This way leaves
/// less room for error, and we can pass all the data to the FVM
/// in one go.
pub struct Validator {
    pub public_key: libsecp256k1::PublicKey,
    pub power: Power,
}

pub struct Genesis {
    pub validators: Vec<Validator>,
    pub accounts: Vec<Actor>,
}
