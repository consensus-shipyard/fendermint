// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, Context};
use fendermint_abci::ApplicationService;
use fendermint_app::ipc::AppParentFinalityQuery;
use fendermint_app::{App, AppConfig, AppStore};
use fendermint_rocksdb::{blockstore::NamespaceBlockstore, namespaces, RocksDb, RocksDbConfig};
use fendermint_vm_interpreter::{
    bytes::{BytesMessageInterpreter, ProposalPrepareMode},
    chain::{ChainMessageInterpreter, CheckpointPool},
    fvm::FvmMessageInterpreter,
    signed::SignedMessageInterpreter,
};
use fendermint_vm_topdown::sync::{launch_polling_syncer, IPCAgentProxy};
use fendermint_vm_topdown::{InMemoryFinalityProvider, MaybeDisabledProvider};
use ipc_sdk::subnet_id::SubnetID;
use std::sync::Arc;
use tracing::info;

use crate::{cmd, options::run::RunArgs, settings::Settings};

cmd! {
  RunArgs(self, settings) {
    run(settings).await
  }
}

fn create_ipc_agent_proxy(
    settings: &fendermint_vm_topdown::Config,
    subnet_id: SubnetID,
) -> anyhow::Result<IPCAgentProxy> {
    let url = settings.ipc_agent_url.parse()?;

    let json_rpc = ipc_agent_sdk::jsonrpc::JsonRpcClientImpl::new(url, None);
    let ipc_agent_client = ipc_agent_sdk::apis::IpcAgentClient::new(json_rpc);
    IPCAgentProxy::new(ipc_agent_client, subnet_id)
}

async fn run(settings: Settings) -> anyhow::Result<()> {
    let interpreter = FvmMessageInterpreter::<NamespaceBlockstore>::new(
        settings.contracts_dir(),
        settings.fvm.gas_overestimation_rate,
        settings.fvm.gas_search_step,
        settings.fvm.exec_in_check,
    );
    let interpreter = SignedMessageInterpreter::new(interpreter);
    let interpreter = ChainMessageInterpreter::new(interpreter);
    let interpreter =
        BytesMessageInterpreter::new(interpreter, ProposalPrepareMode::AppendOnly, false);

    let ns = Namespaces::default();
    let db = open_db(&settings, &ns).context("error opening DB")?;

    let state_store =
        NamespaceBlockstore::new(db.clone(), ns.state_store).context("error creating state DB")?;

    let resolve_pool = CheckpointPool::new();

    let parent_finality_provider = if settings.ipc.is_topdown_enabled() {
        info!("topdown finality enabled");
        let config = settings.ipc.top_down_config()?.clone();
        Arc::new(MaybeDisabledProvider::enabled(
            InMemoryFinalityProvider::uninitialized(config),
        ))
    } else {
        info!("topdown finality disabled");
        Arc::new(MaybeDisabledProvider::disabled())
    };

    let app: App<_, _, AppStore, _, _> = App::new(
        AppConfig {
            app_namespace: ns.app,
            state_hist_namespace: ns.state_hist,
            state_hist_size: settings.db.state_hist_size,
            builtin_actors_bundle: settings.builtin_actors_bundle(),
        },
        db,
        state_store,
        interpreter,
        resolve_pool,
        parent_finality_provider.clone(),
    )?;

    if settings.ipc.is_topdown_enabled() {
        let topdown_config = settings.ipc.top_down_config()?;
        let app_parent_finality_query = AppParentFinalityQuery::new(app.clone());
        launch_polling_syncer(
            &app_parent_finality_query,
            topdown_config.clone(),
            parent_finality_provider,
            create_ipc_agent_proxy(topdown_config, settings.ipc.subnet_id.clone())?,
        )
        .await?;
    }

    let service = ApplicationService(app);

    // Split it into components.
    let (consensus, mempool, snapshot, info) =
        tower_abci::split::service(service, settings.abci.bound);

    // Hand those components to the ABCI server. This is where tower layers could be added.
    let server = tower_abci::v037::Server::builder()
        .consensus(consensus)
        .snapshot(snapshot)
        .mempool(mempool)
        .info(info)
        .finish()
        .context("error creating ABCI server")?;

    // Run the ABCI server.
    server
        .listen(settings.abci.listen.addr())
        .await
        .map_err(|e| anyhow!("error listening: {e}"))?;

    Ok(())
}

namespaces! {
    Namespaces {
        app,
        state_hist,
        state_store
    }
}

/// Open database with all
fn open_db(settings: &Settings, ns: &Namespaces) -> anyhow::Result<RocksDb> {
    let path = settings.data_dir().join("rocksdb");
    info!(
        path = path.to_string_lossy().into_owned(),
        "opening database"
    );
    let db = RocksDb::open_cf(path, &RocksDbConfig::default(), ns.values().iter())?;
    Ok(db)
}
