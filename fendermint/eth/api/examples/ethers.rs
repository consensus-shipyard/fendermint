// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

//! Example of using the Ethereum JSON-RPC facade with the Ethers provider.
//!
//! The example assumes that the following has been started and running in the background:
//! 1. Fendermint ABCI application
//! 2. Tendermint Core / Comet BFT
//! 3. Fendermint Ethereum API facade
//!
//! # Usage
//! ```text
//! cargo run -p fendermint_eth_api --release --example ethers --
//! ```
//!
//! A method can also be called directly with `curl`:
//!
//! ```text
//! curl -X POST -i \
//!      -H 'Content-Type: application/json' \
//!      -d '{"jsonrpc":"2.0","id":0,"method":"eth_getBlockTransactionCountByNumber","params":["0x1"]}' \
//!      http://localhost:8545
//! ```

// See https://coinsbench.com/ethereum-with-rust-tutorial-part-1-create-simple-transactions-with-rust-26d365a7ea93
// and https://coinsbench.com/ethereum-with-rust-tutorial-part-2-compile-and-deploy-solidity-contract-with-rust-c3cd16fce8ee
// and https://coinsbench.com/ethers-rust-power-or-ethers-abigen-rundown-89ab5e47875d

use std::{
    fmt::{Debug, Display},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context};
use clap::Parser;
use ethers::providers::StreamExt;
use ethers::{
    prelude::{abigen, ContractCall, ContractFactory, SignerMiddleware},
    providers::{FilterKind, Http, JsonRpcClient, Middleware, Provider, Ws},
    signers::{Signer, Wallet},
};
use ethers_core::{
    abi::Abi,
    k256::ecdsa::SigningKey,
    types::{
        transaction::eip2718::TypedTransaction, Address, BlockId, BlockNumber, Bytes,
        Eip1559TransactionRequest, Filter, Log, SyncingStatus, TransactionReceipt, H160, H256,
        U256, U64,
    },
};
use fendermint_rpc::message::MessageFactory;
use fendermint_vm_actor_interface::eam::EthAddress;
use fvm_shared::{bigint::BigInt, econ::TokenAmount};
use libsecp256k1::SecretKey;
use tracing::Level;

type TestMiddleware<C> = SignerMiddleware<Provider<C>, Wallet<SigningKey>>;
type TestContractCall<C, T> = ContractCall<TestMiddleware<C>, T>;

// This assumes that https://github.com/filecoin-project/builtin-actors is checked out next to this project,
// which the Makefile in the root takes care of with `make actor-bundle`, a dependency of creating docker images.
const SIMPLECOIN_HEX: &'static str =
    include_str!("../../../../../builtin-actors/actors/evm/tests/contracts/SimpleCoin.bin");

// const SIMPLECOIN_ABI: &'static str =
//     include_str!("../../../../../builtin-actors/actors/evm/tests/contracts/SimpleCoin.abi");

/// Gas limit to set for transactions.
const ENOUGH_GAS: u64 = 10_000_000_000u64;

// Generate a statically typed interface for the contract.
// An example of what it looks like is at https://github.com/filecoin-project/ref-fvm/blob/evm-integration-tests/testing/integration/tests/evm/src/simple_coin/simple_coin.rs
abigen!(
    SimpleCoin,
    "../../../../builtin-actors/actors/evm/tests/contracts/SimpleCoin.abi"
);

#[derive(Parser, Debug)]
pub struct Options {
    /// The host of the Fendermint Ethereum API endpoint.
    #[arg(long, default_value = "127.0.0.1", env = "FM_ETH__LISTEN__HOST")]
    pub http_host: String,

    /// The port of the Fendermint Ethereum API endpoint.
    #[arg(long, default_value = "8545", env = "FM_ETH__LISTEN__PORT")]
    pub http_port: u32,

    /// Secret key used to send funds, expected to be in Base64 format.
    ///
    /// Assumed to exist with a non-zero balance.
    #[arg(long, short)]
    pub secret_key_from: PathBuf,

    /// Secret key used to receive funds, expected to be in Base64 format.
    #[arg(long, short)]
    pub secret_key_to: PathBuf,

    /// Enable DEBUG logs.
    #[arg(long, short)]
    pub verbose: bool,
}

impl Options {
    pub fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }

    pub fn http_endpoint(&self) -> String {
        format!("http://{}:{}", self.http_host, self.http_port)
    }

    pub fn ws_endpoint(&self) -> String {
        // Same address but accessed with GET
        format!("ws://{}:{}", self.http_host, self.http_port)
    }
}

/// See the module docs for how to run.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Options = Options::parse();

    tracing_subscriber::fmt()
        .with_max_level(opts.log_level())
        .init();

    tracing::info!("Running the tests over HTTP...");
    let provider = Provider::<Http>::try_from(opts.http_endpoint())?;
    run_http(provider, &opts).await?;

    tracing::info!("Running the tests over WS...");
    let provider = Provider::<Ws>::connect(opts.ws_endpoint()).await?;
    run_ws(provider, &opts).await?;

    Ok(())
}

trait CheckResult {
    fn check_result(&self) -> anyhow::Result<()>;
}

impl CheckResult for bool {
    fn check_result(&self) -> anyhow::Result<()> {
        if *self {
            Ok(())
        } else {
            Err(anyhow!("expected true; got false"))
        }
    }
}

fn request<T, E, F, C>(method: &str, res: Result<T, E>, check: F) -> anyhow::Result<T>
where
    T: Debug,
    F: FnOnce(&T) -> C,
    C: CheckResult,
    E: Display,
{
    tracing::debug!("checking request {method}...");
    match res {
        Ok(value) => match check(&value).check_result() {
            Ok(()) => Ok(value),
            Err(e) => Err(anyhow!("failed to check {method}: {e}:\n{value:?}")),
        },
        Err(e) => Err(anyhow!("failed to call {method}: {e:#}")),
    }
}

struct TestAccount {
    secret_key: SecretKey,
    eth_addr: H160,
}

impl TestAccount {
    pub fn new(sk: &Path) -> anyhow::Result<Self> {
        let sk = MessageFactory::read_secret_key(sk)?;
        let pk = libsecp256k1::PublicKey::from_secret_key(&sk);
        let ea = EthAddress::new_secp256k1(&pk.serialize())?;
        let h = Address::from_slice(&ea.0);

        Ok(Self {
            secret_key: sk,
            eth_addr: h,
        })
    }
}

// The following methods are called by the [`Provider`].
// This is not an exhaustive list of JSON-RPC methods that the API implements, just what the client library calls.
//
// DONE:
// - eth_accounts
// - eth_blockNumber
// - eth_chainId
// - eth_getBalance
// - eth_getUncleCountByBlockHash
// - eth_getUncleCountByBlockNumber
// - eth_getUncleByBlockHashAndIndex
// - eth_getUncleByBlockNumberAndIndex
// - eth_getTransactionCount
// - eth_gasPrice
// - eth_getBlockByHash
// - eth_getBlockByNumber
// - eth_getTransactionByHash
// - eth_getTransactionReceipt
// - eth_feeHistory
// - eth_sendRawTransaction
// - eth_call
// - eth_estimateGas
// - eth_getBlockReceipts
// - eth_getStorageAt
// - eth_getCode
// - eth_syncing
// - web3_clientVersion
// - eth_getLogs
// - eth_newFilter
// - eth_newBlockFilter
// - eth_newPendingTransactionFilter
// - eth_getFilterChanges
// - eth_uninstallFilter
// - eth_subscribe
// - eth_unsubscribe
//
// DOING:
//
// TODO:
//
// WON'T DO:
// - eth_sign
// - eth_sendTransaction
// - eth_mining
// - eth_createAccessList
// - eth_getProof
//

/// Exercise the above methods, so we know at least the parameters are lined up correctly.
async fn run<C>(provider: &Provider<C>, opts: &Options) -> anyhow::Result<()>
where
    C: JsonRpcClient + Clone + 'static,
{
    let from = TestAccount::new(&opts.secret_key_from)?;
    let to = TestAccount::new(&opts.secret_key_to)?;

    tracing::info!(from = ?from.eth_addr, to = ?to.eth_addr, "ethereum address");

    // Set up filters to collect events.
    let mut filter_ids = Vec::new();

    let logs_filter_id = request(
        "eth_newFilter",
        provider
            .new_filter(FilterKind::Logs(&Filter::default()))
            .await,
        |_| true,
    )?;
    filter_ids.push(logs_filter_id);

    let blocks_filter_id = request(
        "eth_newBlockFilter",
        provider.new_filter(FilterKind::NewBlocks).await,
        |id| *id != logs_filter_id,
    )?;
    filter_ids.push(blocks_filter_id);

    let txs_filter_id = request(
        "eth_newPendingTransactionFilter",
        provider.new_filter(FilterKind::PendingTransactions).await,
        |id| *id != logs_filter_id,
    )?;
    filter_ids.push(txs_filter_id);

    request("web3_clientVersion", provider.client_version().await, |v| {
        v.starts_with("fendermint/")
    })?;

    request("net_version", provider.get_net_version().await, |v| {
        !v.is_empty() && v.chars().all(|c| c.is_numeric())
    })?;

    request("eth_accounts", provider.get_accounts().await, |acnts| {
        acnts.is_empty()
    })?;

    let bn = request("eth_blockNumber", provider.get_block_number().await, |bn| {
        bn.as_u64() > 0
    })?;

    // Go back one block, so we can be sure there are results.
    let bn = bn - 1;

    let chain_id = request("eth_chainId", provider.get_chainid().await, |id| {
        !id.is_zero()
    })?;

    let mw = make_middleware(provider.clone(), chain_id.as_u64(), &from)
        .context("failed to create middleware")?;
    let mw = Arc::new(mw);

    request(
        "eth_getBalance",
        provider.get_balance(from.eth_addr, None).await,
        |b| !b.is_zero(),
    )?;

    request(
        "eth_getUncleCountByBlockHash",
        provider
            .get_uncle_count(BlockId::Hash(H256([0u8; 32])))
            .await,
        |uc| uc.is_zero(),
    )?;

    request(
        "eth_getUncleCountByBlockNumber",
        provider
            .get_uncle_count(BlockId::Number(BlockNumber::Number(bn)))
            .await,
        |uc| uc.is_zero(),
    )?;

    request(
        "eth_getUncleByBlockHashAndIndex",
        provider
            .get_uncle(BlockId::Hash(H256([0u8; 32])), U64::from(0))
            .await,
        |u| u.is_none(),
    )?;

    request(
        "eth_getUncleByBlockNumberAndIndex",
        provider
            .get_uncle(BlockId::Number(BlockNumber::Number(bn)), U64::from(0))
            .await,
        |u| u.is_none(),
    )?;

    // Querying at genesis, so the transaction count should be zero.
    request(
        "eth_getTransactionCount",
        provider
            .get_transaction_count(from.eth_addr, Some(BlockId::Number(BlockNumber::Earliest)))
            .await,
        |u| u.is_zero(),
    )?;

    // Get a block without transactions
    let b = request(
        "eth_getBlockByNumber w/o txns",
        provider
            .get_block(BlockId::Number(BlockNumber::Number(bn)))
            .await,
        |b| b.is_some() && b.as_ref().map(|b| b.number).flatten() == Some(bn),
    )?;

    let bh = b.unwrap().hash.expect("hash should be set");

    // Get the same block without transactions by hash.
    request(
        "eth_getBlockByHash w/o txns",
        provider.get_block(BlockId::Hash(bh)).await,
        |b| b.is_some() && b.as_ref().map(|b| b.number).flatten() == Some(bn),
    )?;

    let base_fee = request("eth_gasPrice", provider.get_gas_price().await, |id| {
        !id.is_zero()
    })?;

    tracing::info!("sending example transfer");

    let transfer = make_transfer(&mw, &to)
        .await
        .context("failed to make a transfer")?;

    let receipt = send_transaction(&mw, transfer.clone(), "transfer")
        .await
        .context("failed to send transfer")?;

    let tx_hash = receipt.transaction_hash;
    let bn = receipt.block_number.unwrap();
    let bh = receipt.block_hash.unwrap();

    tracing::info!(height = ?bn, ?tx_hash, "example transfer");

    // Get a block with transactions by number.
    let block = request(
        "eth_getBlockByNumber w/ txns",
        provider
            .get_block_with_txs(BlockId::Number(BlockNumber::Number(bn)))
            .await,
        |b| b.is_some() && b.as_ref().map(|b| b.number).flatten() == Some(bn),
    )?;

    assert_eq!(
        block.unwrap().transactions[0].hash,
        tx_hash,
        "computed hash should match"
    );

    // Get the block with transactions by hash.
    request(
        "eth_getBlockByHash w/ txns",
        provider.get_block_with_txs(BlockId::Hash(bh)).await,
        |b| b.is_some() && b.as_ref().map(|b| b.number).flatten() == Some(bn),
    )?;

    // By now there should be a transaction in a block.
    request(
        "eth_feeHistory",
        provider
            .fee_history(
                U256::from(100),
                BlockNumber::Latest,
                &[0.25, 0.5, 0.75, 0.95],
            )
            .await,
        |hist| {
            hist.base_fee_per_gas.len() > 0
                && *hist.base_fee_per_gas.last().unwrap() == base_fee
                && hist.gas_used_ratio.iter().any(|r| *r > 0.0)
        },
    )?;

    request(
        "eth_getTransactionByHash",
        provider.get_transaction(tx_hash).await,
        |tx| tx.is_some(),
    )?;

    request(
        "eth_getTransactionReceipt",
        provider.get_transaction_receipt(tx_hash).await,
        |tx| tx.is_some(),
    )?;

    request(
        "eth_getBlockReceipts",
        provider.get_block_receipts(BlockNumber::Number(bn)).await,
        |rs| !rs.is_empty(),
    )?;

    // Calling with 0 nonce so the node figures out the latest value.
    let mut probe_tx = transfer.clone();
    probe_tx.set_nonce(0);
    let probe_height = BlockId::Number(BlockNumber::Number(bn));

    request(
        "eth_call",
        provider.call(&probe_tx, Some(probe_height)).await,
        |_| true,
    )?;

    request(
        "eth_estimateGas w/ height",
        provider.estimate_gas(&probe_tx, Some(probe_height)).await,
        |gas: &U256| !gas.is_zero(),
    )?;

    request(
        "eth_estimateGas w/o height",
        provider.estimate_gas(&probe_tx, None).await,
        |gas: &U256| !gas.is_zero(),
    )?;

    request(
        "eth_estimateGas w/o height",
        provider.estimate_gas(&probe_tx, None).await,
        |gas: &U256| !gas.is_zero(),
    )?;

    request(
        "eth_getBlockByHash w/o txns",
        provider.get_block(BlockId::Hash(bh)).await,
        |b| b.is_some() && b.as_ref().map(|b| b.number).flatten() == Some(bn),
    )?;

    request(
        "eth_maxPriorityFeePerGas",
        test_max_priority_fee_call(opts).await,
        |premium: &TokenAmount| !premium.is_zero(),
    )?;

    tracing::info!("deploying SimpleCoin");

    let bytecode =
        Bytes::from(hex::decode(SIMPLECOIN_HEX).context("failed to decode contract hex")?);

    // let abi = serde_json::from_str::<ethers::core::abi::Abi>(SIMPLECOIN_ABI)?;
    let abi: Abi = SIMPLECOIN_ABI.clone();

    let factory = ContractFactory::new(abi, bytecode.clone(), mw.clone());
    let mut deployer = factory.deploy(())?;

    // Fill the fields so we can debug any difference between this and the node.
    // Using `Some` block ID because with `None` the eth_estimateGas call would receive invalid parameters.
    mw.fill_transaction(&mut deployer.tx, Some(BlockId::Number(BlockNumber::Latest)))
        .await
        .context("failed to fill deploy transaction")?;

    tracing::info!(sighash = ?deployer.tx.sighash(), "deployment tx");

    // NOTE: This will call eth_estimateGas to figure out how much gas to use, because we don't set it,
    // unlike in the case of the example transfer. What the [Provider::fill_transaction] will _also_ do
    // is estimate the fees using eth_feeHistory, here:
    // https://github.com/gakonst/ethers-rs/blob/df165b84229cdc1c65e8522e0c1aeead3746d9a8/ethers-providers/src/rpc/provider.rs#LL300C30-L300C51
    // These were set to zero in the earlier example transfer, ie. it was basically paid for by the miner (which is not at the moment charged),
    // so the test passed. Here, however, there will be a non-zero cost to pay by the deployer, and therefore those balances
    // have to be much higher than the defaults used earlier, e.g. the deployment cost 30 FIL, and we used to give 1 FIL.
    let (contract, deploy_receipt): (_, TransactionReceipt) = deployer
        .send_with_receipt()
        .await
        .context("failed to send deployment")?;

    tracing::info!(addr = ?contract.address(), "SimpleCoin deployed");

    let contract = SimpleCoin::new(contract.address(), contract.client());

    let coin_balance: TestContractCall<_, U256> =
        prepare_call(&mw, contract.get_balance(from.eth_addr)).await?;

    request("eth_call", coin_balance.call().await, |coin_balance| {
        *coin_balance == U256::from(10000)
    })?;

    // We could calculate the storage location of the balance of the owner of the contract,
    // but let's just see what it returns with at slot 0. See an example at
    // https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getstorageat
    request(
        "eth_getStorageAt",
        mw.get_storage_at(
            contract.address(),
            {
                let mut bz = [0u8; 32];
                U256::zero().to_big_endian(&mut bz);
                H256::from_slice(&bz)
            },
            None,
        )
        .await,
        |_| true,
    )?;

    request(
        "eth_code",
        mw.get_code(contract.address(), None).await,
        |bz| {
            // It's not exactly the same as the bytecode above.
            // The initcode is stripped, only the runtime bytecode is returned.
            // But the two seem to start and end the same way.
            let a = bz.iter();
            let b = bytecode.iter();
            let ar = bz.iter().rev();
            let br = bytecode.iter().rev();
            !bz.is_empty()
                && a.zip(b).take_while(|(a, b)| a == b).count() > 0
                && ar.zip(br).take_while(|(a, b)| a == b).count() > 0
        },
    )?;

    request("eth_syncing", mw.syncing().await, |s| {
        *s == SyncingStatus::IsFalse // There is only one node.
    })?;

    // Send a SimpleCoin transaction to get an event emitted.
    // Not using `prepare_call` here because `send_transaction` will fill the missing fields.
    let coin_send_value = U256::from(100);
    let coin_send: TestContractCall<_, bool> = contract.send_coin(to.eth_addr, coin_send_value);
    // Using `send_transaction` instead of `coin_send.send()` so it gets the receipt.
    // Unfortunately the returned `bool` is not available through the Ethereum API.
    let receipt = request(
        "eth_sendRawTransaction",
        send_transaction(&mw, coin_send.tx, "coin_send").await,
        |receipt| !receipt.logs.is_empty(),
    )?;

    request(
        "eth_getLogs",
        mw.get_logs(&Filter::new().at_block_hash(receipt.block_hash.unwrap()))
            .await,
        |logs| *logs == receipt.logs,
    )?;

    // See what kind of events were logged.
    request(
        "eth_getFilterChanges (blocks)",
        mw.get_filter_changes(blocks_filter_id).await,
        |block_hashes: &Vec<H256>| {
            [bh, deploy_receipt.block_hash.unwrap()]
                .iter()
                .all(|h| block_hashes.contains(h))
        },
    )?;

    request(
        "eth_getFilterChanges (txs)",
        mw.get_filter_changes(txs_filter_id).await,
        |tx_hashes: &Vec<H256>| {
            [&tx_hash, &deploy_receipt.transaction_hash]
                .iter()
                .all(|h| tx_hashes.contains(h))
        },
    )?;

    let logs = request(
        "eth_getFilterChanges (logs)",
        mw.get_filter_changes(logs_filter_id).await,
        |logs: &Vec<Log>| !logs.is_empty(),
    )?;

    // eprintln!("LOGS = {logs:?}");

    // Parse `Transfer` events from the logs with the SimpleCoin contract.
    // Based on https://github.com/filecoin-project/ref-fvm/blob/evm-integration-tests/testing/integration/tests/fevm_features/common.rs#L616
    //      and https://github.com/filecoin-project/ref-fvm/blob/evm-integration-tests/testing/integration/tests/fevm_features/simple_coin.rs#L26
    //      and https://github.com/filecoin-project/ref-fvm/blob/evm-integration-tests/testing/integration/tests/evm/src/simple_coin/simple_coin.rs#L103

    // The contract has methods like `.transfer_filter()` which allows querying logs, but here we just test parsing to make sure the data is correct.
    let transfer_events = logs
        .into_iter()
        .filter(|log| log.address == contract.address())
        .map(|log| contract.decode_event::<TransferFilter>("Transfer", log.topics, log.data))
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse logs to transfer events")?;

    assert!(!transfer_events.is_empty());
    assert_eq!(transfer_events[0].from, from.eth_addr);
    assert_eq!(transfer_events[0].to, to.eth_addr);
    assert_eq!(transfer_events[0].value, coin_send_value);

    // Uninstall all filters.
    for id in filter_ids {
        request("eth_uninstallFilter", mw.uninstall_filter(id).await, |ok| {
            *ok
        })?;
    }

    Ok(())
}

/// The HTTP interface provides JSON-RPC request/response endpoints.
async fn run_http(mut provider: Provider<Http>, opts: &Options) -> anyhow::Result<()> {
    adjust_provider(&mut provider);
    run(&provider, opts).await
}

/// The WebSocket interface provides JSON-RPC request/response interactions
/// as well as subscriptions, both using messages over the socket.
///
/// We subscribe to notifications first, then run the same suite of request/responses
/// as the HTTP case, finally check that we have collected events over the subscriptions.
async fn run_ws(mut provider: Provider<Ws>, opts: &Options) -> anyhow::Result<()> {
    adjust_provider(&mut provider);

    // Subscriptions as well.
    let mut block_sub = provider.subscribe_blocks().await?;
    let mut txs_sub = provider.subscribe_pending_txs().await?;
    let mut log_sub = provider.subscribe_logs(&Filter::default()).await?;

    run(&provider, opts).await?;

    assert!(block_sub.next().await.is_some(), "blocks should arrive");
    assert!(txs_sub.next().await.is_some(), "transactions should arrive");
    assert!(log_sub.next().await.is_some(), "logs should arrive");

    block_sub
        .unsubscribe()
        .await
        .context("failed to unsubscribe blocks")?;

    txs_sub
        .unsubscribe()
        .await
        .context("failed to unsubscribe txs")?;

    log_sub
        .unsubscribe()
        .await
        .context("failed to unsubscribe logs")?;

    Ok(())
}

fn adjust_provider<C>(provider: &mut Provider<C>)
where
    C: JsonRpcClient,
{
    // Tendermint block interval is lower.
    provider.set_interval(Duration::from_secs(2));
}

async fn make_transfer<C>(
    mw: &TestMiddleware<C>,
    to: &TestAccount,
) -> anyhow::Result<TypedTransaction>
where
    C: JsonRpcClient + 'static,
{
    // Create a transaction to transfer 1000 atto.
    let tx = Eip1559TransactionRequest::new().to(to.eth_addr).value(1000);

    // Set the gas based on the testkit so it doesn't trigger estimation.
    let mut tx = tx
        .gas(ENOUGH_GAS)
        .max_fee_per_gas(0)
        .max_priority_fee_per_gas(0)
        .into();

    // Fill in the missing fields like `from` and `nonce` (which involves querying the API).
    mw.fill_transaction(&mut tx, None).await?;

    Ok(tx)
}

/// Send a transaction and await the receipt.
async fn send_transaction<C>(
    mw: &TestMiddleware<C>,
    tx: TypedTransaction,
    label: &str,
) -> anyhow::Result<TransactionReceipt>
where
    C: JsonRpcClient + 'static,
{
    // `send_transaction` will fill in the missing fields like `from` and `nonce` (which involves querying the API).
    let receipt = mw
        .send_transaction(tx, None)
        .await
        .context("failed to send transaction")?
        .log_msg(format!("Pending transaction: {label}"))
        .retries(5)
        .await?
        .context("Missing receipt")?;

    Ok(receipt)
}

/// Create a middleware that will assign nonces and sign the message.
fn make_middleware<C>(
    provider: Provider<C>,
    chain_id: u64,
    sender: &TestAccount,
) -> anyhow::Result<TestMiddleware<C>>
where
    C: JsonRpcClient,
{
    // We have to use Ethereum's signing scheme, beause the `from` is not part of the RLP representation,
    // it is inferred from the public key recovered from the signature. We could potentially hash the
    // transaction in a different way, but we can't for example use the actor ID in the hash, because
    // we have no way of sending it along with the message.
    let wallet: Wallet<SigningKey> =
        Wallet::from_bytes(&sender.secret_key.serialize())?.with_chain_id(chain_id);

    Ok(SignerMiddleware::new(provider, wallet))
}

/// Fill the transaction fields such as gas and nonce.
async fn prepare_call<C, T>(
    mw: &TestMiddleware<C>,
    mut call: TestContractCall<C, T>,
) -> anyhow::Result<TestContractCall<C, T>>
where
    C: JsonRpcClient + 'static,
{
    mw.fill_transaction(&mut call.tx, Some(BlockId::Number(BlockNumber::Latest)))
        .await
        .context("failed to fill transaction")?;

    Ok(call)
}

/// Performs an `eth_maxPriorityFeePerGas` call to the ETH RPC
/// This method is not provided by `ethers`, so we have to perform
/// it manually.
async fn test_max_priority_fee_call(opts: &Options) -> anyhow::Result<TokenAmount> {
    let client = reqwest::Client::new();
    let url = opts.http_endpoint();

    let request_data = r#"{
        "jsonrpc": "2.0",
        "id": 0,
        "method": "eth_maxPriorityFeePerGas",
        "params": []
    }"#;

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(request_data)
        .send()
        .await?;

    let response_text: String = response.text().await?;
    let parsed_response: serde_json::Value = serde_json::from_str(&response_text)?;

    if let Some(res) = parsed_response["result"].as_str() {
        if let Ok(val) = u64::from_str_radix(&res[2..], 16) {
            return Ok(TokenAmount::from_atto(BigInt::from(val)));
        } else {
            return Err(anyhow::anyhow!("fail to convert result to hex"));
        }
    }

    Err(anyhow::anyhow!("call to eth_maxPriorityFeePerGas failed"))
}
