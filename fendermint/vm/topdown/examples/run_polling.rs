use fendermint_vm_topdown::{
    Config, IPCAgentProxy, IPCParentFinality, InMemoryFinalityProvider, PollingParentSyncer,
};
use ipc_agent_sdk::apis::IpcAgentClient;
use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
use ipc_sdk::subnet_id::SubnetID;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();

    let url = std::env::var("URL").unwrap_or_else(|_| "http://0.0.0.0:3030/json_rpc".to_string());
    let raw_parent_subnet_id =
        std::env::var("PARENT").unwrap_or_else(|_| "/r31415926".to_string());
    let raw_child_subnet_id =
        std::env::var("CHILD").unwrap_or_else(|_| "/r31415926".to_string());
    let parent_subnet = SubnetID::from_str(&raw_parent_subnet_id).unwrap();
    let child_subnet = SubnetID::from_str(&raw_child_subnet_id).unwrap();

    let json_rpc = JsonRpcClientImpl::new(url.parse().unwrap(), None);
    let ipc_agent_client = IpcAgentClient::new(json_rpc);
    let agent_proxy = IPCAgentProxy::new(ipc_agent_client, parent_subnet, child_subnet);

    let chain_head = agent_proxy.get_chain_head_height().await.unwrap();

    let config = Config {
        chain_head_delay: 10,
        polling_interval_secs: 5,
    };
    let provider = InMemoryFinalityProvider::new(
        config.clone(),
        IPCParentFinality {
            height: chain_head - 20,
            block_hash: vec![],
        },
    );
    let provider = Arc::new(provider);
    let agent = Arc::new(agent_proxy);
    let polling = PollingParentSyncer::new(config, provider.clone(), agent.clone());

    let handle = tokio::spawn(async move {
        polling.start().unwrap();
    });

    tokio::time::sleep(Duration::new(100, 0)).await;
    handle.abort();
}
