// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use cid::{multihash, multihash::MultihashDigest, Cid};
use fvm_ipld_encoding::{to_vec, Error as IpldError, DAG_CBOR};
use serde::Serialize;

mod chain_message;
mod signed_message;

pub use chain_message::ChainMessage;
pub use signed_message::{SignedMessage, SignedMessageError};

/// Calculate the CID using Blake2b256 digest and DAG_CBOR.
///
/// This used to be part of the `Cbor` trait, which is deprecated.
fn cid<T: Serialize>(value: &T) -> Result<Cid, IpldError> {
    let bz = to_vec(value)?;
    let digest = multihash::Code::Blake2b256.digest(&bz);
    let cid = Cid::new_v1(DAG_CBOR, digest);
    Ok(cid)
}
