// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use fvm_shared::address::Address;
use fvm_shared::bigint::BigInt;
use fvm_shared::econ::TokenAmount;
use num_traits::Num;
use serde::de::Error;
use serde::{de, Deserialize, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use std::str::FromStr;

use cid::Cid;

pub struct IsHumanReadable;

impl SerializeAs<Address> for IsHumanReadable {
    fn serialize_as<S>(source: &Address, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            source.to_string().serialize(serializer)
        } else {
            source.serialize(serializer)
        }
    }
}

impl<'de> DeserializeAs<'de, Address> for IsHumanReadable {
    fn deserialize_as<D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match Address::from_str(&s) {
                Ok(a) => Ok(a),
                Err(e) => Err(D::Error::custom(format!(
                    "error deserializing address: {}",
                    e
                ))),
            }
        } else {
            Address::deserialize(deserializer)
        }
    }
}

impl SerializeAs<TokenAmount> for IsHumanReadable {
    /// Serialize tokens as human readable string.
    fn serialize_as<S>(tokens: &TokenAmount, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            tokens.atto().to_str_radix(10).serialize(serializer)
        } else {
            tokens.serialize(serializer)
        }
    }
}

impl<'de> DeserializeAs<'de, TokenAmount> for IsHumanReadable {
    /// Deserialize tokens from human readable decimal format.
    fn deserialize_as<D>(deserializer: D) -> Result<TokenAmount, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match BigInt::from_str_radix(&s, 10) {
                Ok(a) => Ok(TokenAmount::from_atto(a)),
                Err(e) => Err(D::Error::custom(format!(
                    "error deserializing tokens: {}",
                    e
                ))),
            }
        } else {
            TokenAmount::deserialize(deserializer)
        }
    }
}

impl SerializeAs<Cid> for IsHumanReadable {
    /// Serialize tokens as human readable string.
    fn serialize_as<S>(cid: &Cid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            cid.to_string().serialize(serializer)
        } else {
            cid.serialize(serializer)
        }
    }
}

impl<'de> DeserializeAs<'de, Cid> for IsHumanReadable {
    /// Deserialize CID from human readable string.
    fn deserialize_as<D>(deserializer: D) -> Result<Cid, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Cid::from_str(&s)
                .map_err(|e| D::Error::custom(format!("error deserializing CID: {}", e)))
        } else {
            Cid::deserialize(deserializer)
        }
    }
}
