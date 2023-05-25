// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::anyhow;
use axum::routing::post;
use jsonrpc_v2::Data;
use std::{net::ToSocketAddrs, sync::Arc};
use tendermint_rpc::HttpClient;

mod apis;
mod rpc_http_handler;

type JsonRpcServer = Arc<jsonrpc_v2::Server<jsonrpc_v2::MapRouter>>;
type JsonRpcState = Arc<HttpClient>;

/// Start listening to JSON-RPC requests.
pub async fn listen<A: ToSocketAddrs>(listen_addr: A, client: HttpClient) -> anyhow::Result<()> {
    if let Some(listen_addr) = listen_addr.to_socket_addrs()?.next() {
        let server = make_server(Arc::new(client));
        let router = make_router(server);
        let server = axum::Server::try_bind(&listen_addr)?.serve(router.into_make_service());

        tracing::info!(?listen_addr, "bound Ethereum API");
        server.await?;
        Ok(())
    } else {
        Err(anyhow!("failed to convert to any socket address"))
    }
}

/// Register method handlers with the JSON-RPC server construct.
fn make_server(state: JsonRpcState) -> JsonRpcServer {
    let server = jsonrpc_v2::Server::new().with_data(Data(state));
    let server = apis::register_methods(server);
    server.finish()
}

/// Register routes in the `axum` router to handle JSON-RPC and WebSocket calls.
fn make_router(server: JsonRpcServer) -> axum::Router {
    axum::Router::new()
        //.route("/rpc/v0", get(rpc_ws_handler::handle))
        .route("/rpc/v0", post(rpc_http_handler::handle))
        .with_state(server)
}
