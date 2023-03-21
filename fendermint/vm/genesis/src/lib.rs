// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! A Genesis data structure similar to [genesis.Template](https://github.com/filecoin-project/lotus/blob/v1.20.4/genesis/types.go)
//! in Lotus, which is used to [initialize](https://github.com/filecoin-project/lotus/blob/v1.20.4/chain/gen/genesis/genesis.go) the state tree.

use std::str::FromStr;

use fvm_shared::bigint::BigInt;
use fvm_shared::{address::Address, econ::TokenAmount};
use num_traits::Num;
use serde::de::Error;
use serde::{de, Deserialize, Serialize, Serializer};

/// Wrapper around [`Address`] to provide human readable serialization in JSON format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorAddr(pub Address);

impl Serialize for ActorAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.0.to_string().serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ActorAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match Address::from_str(&s) {
                Ok(a) => Ok(Self(a)),
                Err(e) => Err(D::Error::custom(format!(
                    "error deserializing address: {}",
                    e
                ))),
            }
        } else {
            Address::deserialize(deserializer).map(Self)
        }
    }
}

/// Wrapper around [`TokenAmount`] to provide human readable serialization in JSON format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorBalance(pub TokenAmount);

impl Serialize for ActorBalance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.0.atto().to_str_radix(10).serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ActorBalance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match BigInt::from_str_radix(&s, 10) {
                Ok(a) => Ok(Self(TokenAmount::from_atto(a))),
                Err(e) => Err(D::Error::custom(format!(
                    "error deserializing balance: {}",
                    e
                ))),
            }
        } else {
            TokenAmount::deserialize(deserializer).map(Self)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorMeta {
    Account {
        owner: ActorAddr,
    },
    MultiSig {
        signers: Vec<ActorAddr>,
        threshold: usize,
        vesting_duration: u64,
        vesting_start: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    pub meta: ActorMeta,
    pub balance: ActorBalance,
}

/// Total stake delegated to this validator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    pub public_key: libsecp256k1::PublicKey,
    pub power: Power,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    pub validators: Vec<Validator>,
    pub accounts: Vec<Actor>,
}

#[cfg(feature = "arb")]
mod arb {
    use crate::{Actor, ActorAddr, ActorBalance, ActorMeta, Genesis, Power, Validator};
    use fendermint_testing::arb::{ArbAddress, ArbTokenAmount};
    use quickcheck::{Arbitrary, Gen};
    use rand::{rngs::StdRng, SeedableRng};

    impl Arbitrary for ActorMeta {
        fn arbitrary(g: &mut Gen) -> Self {
            if bool::arbitrary(g) {
                ActorMeta::Account {
                    owner: ActorAddr(ArbAddress::arbitrary(g).0),
                }
            } else {
                let n = usize::arbitrary(g) % 5 + 1;
                let signers = (0..n)
                    .map(|_| ActorAddr(ArbAddress::arbitrary(g).0))
                    .collect();
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
                balance: ActorBalance(ArbTokenAmount::arbitrary(g).0),
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
            let nv = usize::arbitrary(g) % 10 + 1;
            let na = usize::arbitrary(g) % 10;
            Self {
                validators: (0..nv).map(|_| Arbitrary::arbitrary(g)).collect(),
                accounts: (0..na).map(|_| Arbitrary::arbitrary(g)).collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck_macros::quickcheck;

    use crate::Genesis;

    #[quickcheck]
    fn genesis_json(value0: Genesis) {
        let repr = serde_json::to_string(&value0).expect("failed to encode");
        let value1: Genesis = serde_json::from_str(&repr).expect("failed to decode");

        assert_eq!(value1, value0)
    }
}
