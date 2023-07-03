// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Context;
use tendermint::abci;
use tendermint_rpc::Client;

use crate::{JsonRpcData, JsonRpcResult};

/// Returns the current client version.
pub async fn client_version<C>(data: JsonRpcData<C>) -> JsonRpcResult<String>
where
    C: Client + Sync + Send,
{
    let res: abci::response::Info = data
        .tm()
        .abci_info()
        .await
        .context("failed to fetch info")?;

    let version = format!("{}/{}/{}", res.data, res.version, res.app_version);

    Ok(version)
}
