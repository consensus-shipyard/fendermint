// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::marker::PhantomData;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use fendermint_vm_message::chain::ChainMessage;
use tendermint::abci::response::DeliverTx;
use tendermint::block::Height;
use tendermint_rpc::v0_37::Client;
use tendermint_rpc::{endpoint::abci_query::AbciQuery, HttpClient, Scheme, Url};

use fendermint_vm_message::query::FvmQuery;

use crate::message::MessageFactory;
use crate::query::QueryClient;
use crate::tx::{
    AsyncResponse, BoundClient, CommitResponse, SyncResponse, TxAsync, TxClient, TxCommit, TxSync,
};

// Retrieve the proxy URL with precedence:
// 1. If supplied, that's the proxy URL used.
// 2. If not supplied, but environment variable HTTP_PROXY or HTTPS_PROXY are
//    supplied, then use the appropriate variable for the URL in question.
//
// Copied from `tendermint_rpc`.
fn get_http_proxy_url(url_scheme: Scheme, proxy_url: Option<Url>) -> anyhow::Result<Option<Url>> {
    match proxy_url {
        Some(u) => Ok(Some(u)),
        None => match url_scheme {
            Scheme::Http => std::env::var("HTTP_PROXY").ok(),
            Scheme::Https => std::env::var("HTTPS_PROXY")
                .ok()
                .or_else(|| std::env::var("HTTP_PROXY").ok()),
            _ => {
                if std::env::var("HTTP_PROXY").is_ok() || std::env::var("HTTPS_PROXY").is_ok() {
                    tracing::warn!(
                        "Ignoring HTTP proxy environment variables for non-HTTP client connection"
                    );
                }
                None
            }
        }
        .map(|u| u.parse::<Url>().map_err(|e| anyhow!(e)))
        .transpose(),
    }
}

/// Create a Tendermint HTTP client.
pub fn http_client(url: Url, proxy_url: Option<Url>) -> anyhow::Result<HttpClient> {
    let proxy_url = get_http_proxy_url(url.scheme(), proxy_url)?;
    let client = match proxy_url {
        Some(proxy_url) => {
            tracing::debug!(
                "Using HTTP client with proxy {} to submit request to {}",
                proxy_url,
                url
            );
            HttpClient::new_with_proxy(url, proxy_url)?
        }
        None => {
            tracing::debug!("Using HTTP client to submit request to: {}", url);
            HttpClient::new(url)?
        }
    };
    Ok(client)
}

/// Unauthenticated Fendermint client.
pub struct FendermintClient<C: Client = HttpClient> {
    inner: C,
}

impl<C: Client> FendermintClient<C> {
    pub fn new(inner: C) -> Self {
        Self { inner }
    }

    /// Attach a message factory to the client.
    pub fn bind(self, message_factory: MessageFactory) -> BoundFendermintClient<Self, C> {
        BoundFendermintClient::new(self, message_factory)
    }
}

impl FendermintClient<HttpClient> {
    pub fn new_http(url: Url, proxy_url: Option<Url>) -> anyhow::Result<Self> {
        let inner = http_client(url, proxy_url)?;
        Ok(Self { inner })
    }
}

/// Get to the underlying Tendermint client if necessary, for example to query the state of transactions.
pub trait TendermintClient<C: Client> {
    /// The underlying Tendermint client.
    fn underlying(&self) -> &C;
}

impl<C: Client> TendermintClient<C> for FendermintClient<C> {
    fn underlying(&self) -> &C {
        &self.inner
    }
}

#[async_trait]
impl<C> QueryClient for FendermintClient<C>
where
    C: Client + Sync + Send,
{
    async fn perform(&self, query: FvmQuery, height: Option<Height>) -> anyhow::Result<AbciQuery> {
        let data = fvm_ipld_encoding::to_vec(&query).context("failed to encode query")?;

        let res = self.inner.abci_query(None, data, height, false).await?;

        Ok(res)
    }
}

/// Fendermint client capable of signing transactions.
pub struct BoundFendermintClient<I, C> {
    inner: I,
    message_factory: MessageFactory,
    client: PhantomData<C>,
}

impl<I, C> BoundFendermintClient<I, C>
where
    I: TendermintClient<C>,
    C: Client,
{
    pub fn new(inner: I, message_factory: MessageFactory) -> Self {
        Self {
            inner,
            message_factory,
            client: PhantomData,
        }
    }
}

impl<I, C> BoundClient for BoundFendermintClient<I, C> {
    fn message_factory_mut(&mut self) -> &mut MessageFactory {
        &mut self.message_factory
    }
}

impl<I, C> TendermintClient<C> for BoundFendermintClient<I, C>
where
    I: TendermintClient<C>,
    C: Client,
{
    fn underlying(&self) -> &C {
        &self.inner.underlying()
    }
}

#[async_trait]
impl<I, C> QueryClient for BoundFendermintClient<I, C>
where
    I: QueryClient + Sync + Send,
    C: Sync + Send,
{
    async fn perform(&self, query: FvmQuery, height: Option<Height>) -> anyhow::Result<AbciQuery> {
        self.inner.perform(query, height).await
    }
}

#[async_trait]
impl<I, C> TxClient<TxAsync> for BoundFendermintClient<I, C>
where
    I: TendermintClient<C> + Sync + Send,
    C: Client + Sync + Send,
{
    async fn perform<F, T>(&self, msg: ChainMessage, _f: F) -> anyhow::Result<AsyncResponse<T>>
    where
        F: FnOnce(&DeliverTx) -> anyhow::Result<T> + Sync + Send,
    {
        let data = MessageFactory::serialize(&msg)?;
        let response = self.underlying().broadcast_tx_async(data).await?;
        let response = AsyncResponse {
            response,
            return_data: PhantomData,
        };
        Ok(response)
    }
}

#[async_trait]
impl<I, C> TxClient<TxSync> for BoundFendermintClient<I, C>
where
    I: TendermintClient<C> + Sync + Send,
    C: Client + Sync + Send,
{
    async fn perform<F, T>(
        &self,
        msg: ChainMessage,
        _f: F,
    ) -> anyhow::Result<crate::tx::SyncResponse<T>>
    where
        F: FnOnce(&DeliverTx) -> anyhow::Result<T> + Sync + Send,
    {
        let data = MessageFactory::serialize(&msg)?;
        let response = self.underlying().broadcast_tx_sync(data).await?;
        let response = SyncResponse {
            response,
            return_data: PhantomData,
        };
        Ok(response)
    }
}

#[async_trait]
impl<I, C> TxClient<TxCommit> for BoundFendermintClient<I, C>
where
    I: TendermintClient<C> + Sync + Send,
    C: Client + Sync + Send,
{
    async fn perform<F, T>(
        &self,
        msg: ChainMessage,
        f: F,
    ) -> anyhow::Result<crate::tx::CommitResponse<T>>
    where
        F: FnOnce(&DeliverTx) -> anyhow::Result<T> + Sync + Send,
    {
        let data = MessageFactory::serialize(&msg)?;
        let response = self.underlying().broadcast_tx_commit(data).await?;
        let return_data = if response.deliver_tx.code.is_err() {
            None
        } else {
            let return_data = f(&response.deliver_tx).context("error decoding deliver_tx")?;
            Some(return_data)
        };
        let response = CommitResponse {
            response,
            return_data,
        };
        Ok(response)
    }
}
