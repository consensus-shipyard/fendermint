// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fvm_shared::error::ExitCode;

pub struct JsonRpcError {
    code: i64,
    message: String,
}

impl From<anyhow::Error> for JsonRpcError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            code: 0,
            message: format!("{:#}", value),
        }
    }
}

impl From<tendermint_rpc::Error> for JsonRpcError {
    fn from(value: tendermint_rpc::Error) -> Self {
        Self {
            code: 0,
            message: format!("Tendermint RPC error: {value}"),
        }
    }
}

impl From<JsonRpcError> for jsonrpc_v2::Error {
    fn from(value: JsonRpcError) -> Self {
        Self::Full {
            code: value.code,
            message: value.message,
            data: None,
        }
    }
}

pub fn error<T>(exit_code: ExitCode, msg: impl ToString) -> Result<T, JsonRpcError> {
    Err(JsonRpcError {
        code: exit_code.value().into(),
        message: msg.to_string(),
    })
}
