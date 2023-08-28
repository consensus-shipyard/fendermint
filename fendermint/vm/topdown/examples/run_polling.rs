use async_stm::atomically_or_err;
use fendermint_vm_topdown::{
    Config, IPCAgentProxy, IPCParentFinality, InMemoryFinalityProvider, ParentFinalityProvider,
    PollingParentSyncer,
};
use fvm_shared::address::{set_current_network, Network};
use ipc_agent_sdk::apis::IpcAgentClient;
use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
use ipc_sdk::subnet_id::SubnetID;
use num_traits::FromPrimitive;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    set_network_from_env();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let url = std::env::var("URL").unwrap_or_else(|_| "http://0.0.0.0:3030/json_rpc".to_string());
    let raw_target_subnet_id = std::env::var("TARGET").unwrap_or_else(|_| "/r31415926".to_string());
    let subnet = SubnetID::from_str(&raw_target_subnet_id).unwrap();

    let json_rpc = JsonRpcClientImpl::new(url.parse().unwrap(), None);
    let ipc_agent_client = IpcAgentClient::new(json_rpc);
    let agent_proxy = IPCAgentProxy::new(ipc_agent_client, subnet).unwrap();

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
    let polling = PollingParentSyncer::new(config, provider.clone(), agent);

    tokio::spawn(async move {
        polling.start().unwrap();
    });

    loop {
        atomically_or_err(|| {
            let proposal = provider.next_proposal()?;
            println!("proposal: {proposal:?}");
            Ok(())
        })
        .await
        .unwrap();

        tokio::time::sleep(Duration::new(5, 0)).await;
    }
}

pub fn set_network_from_env() {
    let network_raw: u8 = std::env::var("LOTUS_NETWORK")
        // default to testnet
        .unwrap_or_else(|_| String::from("1"))
        .parse()
        .unwrap();
    let network = Network::from_u8(network_raw).unwrap();
    set_current_network(network);
}
