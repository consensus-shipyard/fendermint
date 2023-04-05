// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use tendermint::block::Height;
use tendermint_rpc::v0_37::Client;
use tendermint_rpc::{endpoint::abci_query::AbciQuery, HttpClient, Scheme, Url};

use fendermint_vm_message::query::FvmQuery;

use crate::query::QueryClient;

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
pub struct FendermintClient {
    inner: HttpClient,
}

impl FendermintClient {
    pub fn new(inner: HttpClient) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl QueryClient for FendermintClient {
    /// Run an ABCI query.
    async fn perform(&self, query: FvmQuery, height: Option<Height>) -> anyhow::Result<AbciQuery> {
        let data = fvm_ipld_encoding::to_vec(&query).context("failed to encode query")?;

        let res = self.inner.abci_query(None, data, height, false).await?;

        Ok(res)
    }
}
