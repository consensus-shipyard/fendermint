// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and FVM data formats.

use anyhow::Context;
use ethers_core::types::{transaction::eip2718::TypedTransaction, Eip1559TransactionRequest, H256};

pub use fendermint_vm_message::conv::from_eth::*;
use fvm_shared::{error::ExitCode, message::Message};

use crate::{error, JsonRpcResult};

pub fn to_tm_hash(value: &H256) -> anyhow::Result<tendermint::Hash> {
    tendermint::Hash::try_from(value.as_bytes().to_vec())
        .context("failed to convert to Tendermint Hash")
}

pub fn to_fvm_message(tx: TypedTransaction, accept_legacy: bool) -> JsonRpcResult<Message> {
    match tx {
        TypedTransaction::Eip1559(ref tx) => {
            Ok(fendermint_vm_message::conv::from_eth::to_fvm_message(tx)?)
        }
        TypedTransaction::Legacy(_) if accept_legacy => {
            // legacy transactions are only accepted for gas estimation purposes.
            // eth_sendRawTransaction should fail for legacy transactions.

            let mut tx_1559: Eip1559TransactionRequest = tx.into();
            // We should keep information about gas_premium or not???
            // tx_1559.gas_premium = 0;
            Ok(fendermint_vm_message::conv::from_eth::to_fvm_message(
                &tx_1559,
            )?)
        }
        TypedTransaction::Legacy(_) | TypedTransaction::Eip2930(_) => error(
            ExitCode::USR_ILLEGAL_ARGUMENT,
            "unexpected transaction type",
        ),
    }
}
