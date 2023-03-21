// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A Genesis data structure similar to [genesis.Template](https://github.com/filecoin-project/lotus/blob/v1.20.4/genesis/types.go)
//! in Lotus, which is used to [initialize](https://github.com/filecoin-project/lotus/blob/v1.20.4/chain/gen/genesis/genesis.go) the state tree.

use fvm_shared::{address::Address, econ::TokenAmount};

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Actor {
    pub meta: ActorMeta,
    pub balance: TokenAmount,
}

/// Total stake delegated to this validator.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct Validator {
    pub public_key: libsecp256k1::PublicKey,
    pub power: Power,
}

#[derive(Clone, Debug)]
pub struct Genesis {
    pub validators: Vec<Validator>,
    pub accounts: Vec<Actor>,
}

#[cfg(feature = "arb")]
mod arb {
    use crate::{Actor, ActorMeta, Genesis, Power, Validator};
    use fendermint_testing::arb::{ArbAddress, ArbTokenAmount};
    use quickcheck::{Arbitrary, Gen};
    use rand::{rngs::StdRng, SeedableRng};

    impl Arbitrary for ActorMeta {
        fn arbitrary(g: &mut Gen) -> Self {
            if bool::arbitrary(g) {
                ActorMeta::Account {
                    owner: ArbAddress::arbitrary(g).0,
                }
            } else {
                let n = usize::arbitrary(g) % 5 + 1;
                let signers = (0..n).map(|_| ArbAddress::arbitrary(g).0).collect();
                let threshold = usize::arbitrary(g) % n + 1;
                ActorMeta::MultiSig {
                    signers,
                    threshold,
                    vesting_duration: u64::arbitrary(g),
                    vesting_start: u64::arbitrary(g),
                }
            }
        }
    }

    impl Arbitrary for Actor {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                meta: ActorMeta::arbitrary(g),
                balance: ArbTokenAmount::arbitrary(g).0,
            }
        }
    }

    impl Arbitrary for Validator {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let sk = libsecp256k1::SecretKey::random(&mut rng);
            let pk = libsecp256k1::PublicKey::from_secret_key(&sk);
            Self {
                public_key: pk,
                power: Power(u64::arbitrary(g)),
            }
        }
    }

    impl Arbitrary for Genesis {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                validators: Arbitrary::arbitrary(g),
                accounts: Arbitrary::arbitrary(g),
            }
        }
    }
}
