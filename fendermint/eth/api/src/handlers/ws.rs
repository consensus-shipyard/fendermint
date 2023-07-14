// Copyright 2022-2023 Protocol Labs
// Copyright 2019-2022 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

// Based on https://github.com/ChainSafe/forest/blob/v0.8.2/node/rpc/src/rpc_ws_handler.rs

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use jsonrpc_v2::{RequestObject as JsonRpcRequest, ResponseObject, ResponseObjects};
use serde_json::json;

use crate::{apis, state::WebSocketId, AppState, JsonRpcServer};

/// Similar to [ethers_providers::rpc::transports::ws::types::Notification], which is what the library
/// expects for non-request-response payloads in [PubSubItem::deserialize].
#[derive(Debug)]
pub struct Notification {
    pub method: String,
    pub subscription: ethers_core::types::U256,
    pub result: serde_json::Value,
}

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
    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::unbounded_channel();

    let web_socket_id = state.rpc_state.add_web_socket(notif_tx).await;

    loop {
        tokio::select! {
            Some(Ok(message)) = receiver.next() => {
                handle_incoming(web_socket_id, &state.rpc_server, &mut sender, message).await
            },
            Some(notif) = notif_rx.recv() => {
                handle_outgoing(&mut sender, notif).await
            },
            else => break,
        }
    }

    // Clean up.
    state.rpc_state.remove_web_socket(&web_socket_id).await;
}

/// Handle an incoming request.
async fn handle_incoming(
    web_socket_id: WebSocketId,
    rpc_server: &JsonRpcServer,
    sender: &mut SplitSink<WebSocket, Message>,
    message: Message,
) {
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
                            Some(serde_json::Value::Array(ref mut params)) => params.push(
                                serde_json::Value::Number(serde_json::Number::from(web_socket_id)),
                            ),
                            _ => {
                                tracing::debug!(
                                        "JSON-RPC streaming request has no or unexpected params: {json}"
                                    )
                            }
                        }
                    }

                    match serde_json::from_value::<JsonRpcRequest>(json) {
                        Ok(req) => {
                            send_call_result(rpc_server, sender, req).await;
                        }
                        Err(e) => {
                            deserialization_error("RequestObject", e);
                        }
                    }
                }
                Err(e) => {
                    deserialization_error("JSON", e);
                }
            }
        }
    }
}

fn deserialization_error(what: &str, e: serde_json::Error) {
    // Not responding to the websocket because it requires valid responses, which need to have
    // the `id` field present, which we'd only get if we managed to parse the request.
    // Using `debug!` so someone sending junk cannot flood the log with warnings.
    tracing::debug!("Error deserializing WS payload as {what}: {e}");
}

/// Send a message from the application, result of an async subscription.
async fn handle_outgoing(sender: &mut SplitSink<WebSocket, Message>, notif: Notification) {
    // Based on https://github.com/gakonst/ethers-rs/blob/ethers-v2.0.7/ethers-providers/src/rpc/transports/ws/types.rs#L145
    let json = json! ({
        "method": notif.method,
        "params": {
            "subscription": notif.subscription,
            "result": notif.result
        }
    });

    match serde_json::to_string(&json) {
        Err(e) => {
            tracing::error!(error=?e, "failed to serialize notification to JSON");
        }
        Ok(response) => {
            if let Err(e) = sender.send(Message::Text(response)).await {
                tracing::warn!("failed to send notfication to WS: {e}");
            }
        }
    }
}

/// Call the RPC method and respond through the Web Socket.
async fn send_call_result(
    server: &JsonRpcServer,
    sender: &mut SplitSink<WebSocket, Message>,
    request: jsonrpc_v2::RequestObject,
) {
    let method = request.method_ref();

    tracing::debug!("RPC WS called method: {}", method);

    match server.handle(request).await {
        ResponseObjects::Empty => {}
        ResponseObjects::One(response) => {
            send_response(sender, response).await;
        }
        ResponseObjects::Many(responses) => {
            for response in responses {
                send_response(sender, response).await;
            }
        }
    }
}

async fn send_response(sender: &mut SplitSink<WebSocket, Message>, response: ResponseObject) {
    let response = serde_json::to_string(&response);

    match response {
        Err(e) => {
            tracing::error!(error=?e, "failed to serialize response to JSON");
        }
        Ok(json) => {
            if let Err(e) = sender.send(Message::Text(json)).await {
                tracing::warn!("failed to send response to WS: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jsonrpc_v2::RequestObject;

    #[test]
    fn can_parse_request() {
        let text = "{\"id\":0,\"jsonrpc\":\"2.0\",\"method\":\"eth_newFilter\",\"params\":[{\"topics\":[]}]}";
        let value = serde_json::from_str::<serde_json::Value>(&text).expect("should parse as JSON");
        let _request = serde_json::from_value::<RequestObject>(value)
            .expect("should parse as JSON-RPC request");
    }
}
