// Copyright 2022-2023 Protocol Labs
// Copyright 2019-2022 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

// Based on https://github.com/ChainSafe/forest/blob/v0.8.2/node/rpc/src/rpc_ws_handler.rs

use anyhow::Context;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use fvm_shared::error::ExitCode;
use jsonrpc_v2::RequestObject as JsonRpcRequest;

use crate::{apis, handlers::call_rpc_str, AppState, JsonRpcServer};

pub async fn handle(
    _headers: HeaderMap,
    axum::extract::State(state): axum::extract::State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async { rpc_ws_handler_inner(state, socket).await })
}

/// Handle requests in a loop, interpreting each message as a JSON-RPC request.
///
/// Messages are evaluated one by one. We could spawn tasks like Forest,
/// but there should be some rate limiting applied to avoid DoS attacks.
async fn rpc_ws_handler_inner(state: AppState, socket: WebSocket) {
    tracing::debug!("Accepted WS connection!");
    let (mut sender, mut receiver) = socket.split();

    // Create a channel over which the application can send messages to this socket.
    let (socket_tx, _socket_rx) = tokio::sync::mpsc::unbounded_channel();

    let web_socket_id = state.rpc_state.add_web_socket(socket_tx).await;

    // TODO: Use tokio select!

    while let Some(Ok(message)) = receiver.next().await {
        tracing::debug!("Received new WS RPC message: {:?}", message);

        if let Message::Text(request_text) = message {
            tracing::debug!("WS RPC Request: {}", request_text);

            if !request_text.is_empty() {
                tracing::debug!("RPC Request Received: {:?}", &request_text);

                match serde_json::from_str::<serde_json::Value>(&request_text) {
                    Ok(mut json) => {
                        // If the method requires web sockets, append the ID of the socket to the parameters.
                        let is_streaming = match json.get("method") {
                            Some(serde_json::Value::String(method)) => {
                                apis::is_streaming_method(method)
                            }
                            _ => false,
                        };

                        if is_streaming {
                            match json.get_mut("params") {
                                Some(serde_json::Value::Array(ref mut params)) => {
                                    params.push(serde_json::Value::Number(
                                        serde_json::Number::from(web_socket_id),
                                    ))
                                }
                                _ => {
                                    tracing::debug!(
                                        "JSON-RPC streaming request has no or unexpected params: {json}"
                                    )
                                }
                            }
                        }

                        match serde_json::from_value::<JsonRpcRequest>(json) {
                            Ok(req) => {
                                match rpc_ws_call(&state.rpc_server, &mut sender, req).await {
                                    Ok(()) => {
                                        tracing::debug!("WS RPC task success.");
                                    }
                                    Err(e) => {
                                        tracing::warn!("failed to send response to WS: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                send_error(
                                    &mut sender,
                                    format!(
                                        "Error deserializing WS payload as JSON-RPC request: {e}"
                                    ),
                                )
                                .await;
                            }
                        }
                    }
                    Err(e) => {
                        send_error(
                            &mut sender,
                            format!("Error deserializing WS payload as JSON: {e}"),
                        )
                        .await;
                    }
                }
            }
        }

        state.rpc_state.remove_web_socket(&web_socket_id).await;
    }

    // TODO: Remove web socket.
}

async fn send_error(sender: &mut SplitSink<WebSocket, Message>, msg: String) {
    tracing::error!("{}", msg);
    if let Err(e) = sender
        .send(Message::Text(error_str(
            ExitCode::USR_SERIALIZATION.value() as i64,
            msg,
        )))
        .await
    {
        tracing::warn!("failed to send error response to WS: {e}");
    }
}

/// Call the RPC method and respond through the Web Socket.
async fn rpc_ws_call(
    server: &JsonRpcServer,
    sender: &mut SplitSink<WebSocket, Message>,
    request: jsonrpc_v2::RequestObject,
) -> anyhow::Result<()> {
    let method = request.method_ref();

    tracing::debug!("RPC WS called method: {}", method);

    match call_rpc_str(server, request).await {
        Ok(response) => sender
            .send(Message::Text(response))
            .await
            .context("failed to send success result to WS"),
        Err(e) => {
            tracing::error!("RPC call failed: {}", e);
            sender
                .send(Message::Text(error_str(
                    ExitCode::USR_UNSPECIFIED.value() as i64,
                    e.to_string(),
                )))
                .await
                .context("failed to send error result to WS")
        }
    }
}

pub fn error_res(code: i64, message: String) -> jsonrpc_v2::ResponseObject {
    jsonrpc_v2::ResponseObject::Error {
        jsonrpc: jsonrpc_v2::V2,
        error: jsonrpc_v2::Error::Full {
            code,
            message,
            data: None,
        },
        id: jsonrpc_v2::Id::Null,
    }
}

pub fn error_str(code: i64, message: String) -> String {
    match serde_json::to_string(&error_res(code, message)) {
        Ok(err_str) => err_str,
        Err(err) => format!("Failed to serialize error data. Error was: {err}"),
    }
}
