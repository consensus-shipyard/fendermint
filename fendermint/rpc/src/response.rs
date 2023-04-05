// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use anyhow::{anyhow, Context};
use base64::Engine;
use bytes::Bytes;
use fendermint_vm_actor_interface::eam::{self, CreateReturn};
use fvm_ipld_encoding::BytesDe;
use tendermint::abci::response::DeliverTx;

/// Parse what Tendermint returns in the `data` field of [`DeliverTx`] into bytes.
/// Somewhere along the way it replaces them with the bytes of a Base64 encoded string,
/// and `tendermint_rpc` does not undo that wrapping.
pub fn decode_data(data: &Bytes) -> anyhow::Result<Vec<u8>> {
    let b64 = String::from_utf8(data.to_vec()).context("error parsing data as base64 string")?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .context("error parsing base64 to bytes")?;
    Ok(data)
}

/// Parse what Tendermint returns in the `data` field of [`DeliverTx`] as raw bytes.
pub fn decode_bytes(deliver_tx: &DeliverTx) -> anyhow::Result<Vec<u8>> {
    decode_data(&deliver_tx.data)
}

/// Parse what Tendermint returns in the `data` field of [`DeliverTx`] as [`CreateReturn`].
pub fn decode_fevm_create(deliver_tx: &DeliverTx) -> anyhow::Result<CreateReturn> {
    let data = decode_data(&deliver_tx.data)?;
    fvm_ipld_encoding::from_slice::<eam::CreateReturn>(&data)
        .map_err(|e| anyhow!("error parsing as CreateReturn: {e}"))
}

/// Parse what Tendermint returns in the `data` field of [`DeliverTx`] as raw ABI return value.
pub fn decode_fevm_invoke(deliver_tx: &DeliverTx) -> anyhow::Result<Vec<u8>> {
    let data = decode_data(&deliver_tx.data)?;
    fvm_ipld_encoding::from_slice::<BytesDe>(&data)
        .map(|bz| bz.0)
        .map_err(|e| anyhow!("failed to deserialize bytes: {e}"))
}
