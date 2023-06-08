// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Helper methods to convert between Ethereum and FVM data formats.

use ethers_core::types::Eip1559TransactionRequest;
use fvm_shared::message::Message;

pub fn to_fvm_message(_tx: &Eip1559TransactionRequest) -> Message {
    todo!()
}
