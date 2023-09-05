// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use async_stm::atomically_or_err;
use clap::Parser;
use fendermint_vm_topdown::{
    Config, Error, IPCAgentProxy, IPCParentFinality, InMemoryFinalityProvider,
    ParentFinalityProvider, ParentViewProvider, PollingParentSyncer,
};
use fvm_shared::address::{set_current_network, Network};
use ipc_agent_sdk::apis::IpcAgentClient;
use ipc_agent_sdk::jsonrpc::JsonRpcClientImpl;
use ipc_sdk::subnet_id::SubnetID;
use num_traits::FromPrimitive;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::Level;

#[derive(Parser, Debug)]
pub struct Options {
    /// The URL of the ipc agent's RPC endpoint.
    #[arg(
        long,
        short,
        default_value = "http://127.0.0.1:3030/json_rpc",
        env = "IPC_AGENT_RPC_URL"
    )]
    pub ipc_agent_url: String,

    /// Enable DEBUG logs.
    #[arg(long, short)]
    pub verbose: bool,

    /// The subnet id expressed a string
    #[arg(long, short)]
    pub subnet_id: String,

    /// The subnet id expressed a string
    #[arg(long, short, default_value = "1", env = "LOTUS_NETWORK")]
    pub lotus_network: u8,
}

impl Options {
    pub fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
}

#[tokio::main]
async fn main() {
    let opts: Options = Options::parse();

    set_network(opts.lotus_network);

    tracing_subscriber::fmt()
        .with_max_level(opts.log_level())
        .init();

    let subnet = SubnetID::from_str(&opts.subnet_id).unwrap();
    let json_rpc = JsonRpcClientImpl::new(opts.ipc_agent_url.parse().unwrap(), None);
    let ipc_agent_client = IpcAgentClient::new(json_rpc);
    let agent_proxy = IPCAgentProxy::new(ipc_agent_client, subnet).unwrap();

    let config = Config {
        chain_head_delay: 10,
        polling_interval_secs: 5,
        ipc_agent_url: url.clone(),
    };
    let chain_head = agent_proxy.get_chain_head_height().await.unwrap();
    // Mocked committed finality as we dont have a contract to store the parent finality
    let mocked_committed_finality = IPCParentFinality {
        height: chain_head - 20,
        block_hash: vec![0; 32],
    };
    let provider = InMemoryFinalityProvider::new(config.clone(), Some(mocked_committed_finality));
    let provider = Arc::new(provider);
    let agent = Arc::new(agent_proxy);
    let polling = PollingParentSyncer::new(config, provider.clone(), agent);

    tokio::spawn(async move {
        polling.start().unwrap();
    });

    loop {
        let maybe_proposal = atomically_or_err::<_, Error, _>(|| {
            let proposal = provider.next_proposal()?;
            if let Some(p) = proposal {
                let msgs = provider.top_down_msgs(p.height)?;
                return Ok(Some((p, msgs)));
            }
            Ok(None)
        })
        .await;

        match maybe_proposal {
            Ok(Some((proposal, msgs))) => {
                println!("proposal: {proposal:?}");
                println!("topdown messages: {:?}", msgs);
            }
            Ok(None) => {}
            Err(Error::HeightNotReady) => {
                println!("polling not started yet");
            }
            _ => unreachable!(),
        }

        tokio::time::sleep(Duration::new(5, 0)).await;
    }
}

pub fn set_network(network: u8) {
    let network = Network::from_u8(network).unwrap();
    set_current_network(network);
}
