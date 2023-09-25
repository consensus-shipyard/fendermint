// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{anyhow, bail, Context};
use ethers::types as et;
use fendermint_rpc::client::FendermintClient;
use fendermint_rpc::query::QueryClient;
use fendermint_vm_actor_interface::evm;
use fendermint_vm_message::{chain::ChainMessage, query::FvmQueryHeight, signed::SignedMessage};
use fvm_ipld_encoding::{BytesSer, RawBytes};
use fvm_shared::{address::Address, chainid::ChainID, econ::TokenAmount, BLOCK_GAS_LIMIT};
use libsecp256k1::SecretKey;
use tendermint_rpc::Client;

/// Broadcast transactions to Tendermint.
///
/// This is typically something only active validators would want to do
/// from within Fendermint as part of the block lifecycle, for example
/// to submit their signatures to the ledger.
#[derive(Clone)]
pub struct Broadcaster<C> {
    client: FendermintClient<C>,
    addr: Address,
    secret_key: SecretKey,
    gas_fee_cap: TokenAmount,
    gas_premium: TokenAmount,
}

impl<C> Broadcaster<C>
where
    C: Client + Send + Sync,
{
    pub fn new(
        _client: C,
        _secret_key: SecretKey,
        _gas_fee_cap: TokenAmount,
        _gas_premium: TokenAmount,
    ) -> Self {
        todo!()
    }

    pub async fn fevm_invoke(
        &self,
        to: Address,
        calldata: et::Bytes,
        chain_id: &ChainID,
    ) -> anyhow::Result<()> {
        let params = RawBytes::serialize(BytesSer(&calldata))?;

        let mut message = fvm_shared::message::Message {
            version: Default::default(),
            from: self.addr,
            to,
            sequence: 0,
            value: TokenAmount::from_whole(0),
            method_num: evm::Method::InvokeContract as u64,
            params,
            gas_limit: BLOCK_GAS_LIMIT,
            // TODO: Maybe we should implement something like the Ethereum facade for estimating fees?
            // I don't want to call the Ethereum API directly (it would be one more dependency).
            // Another option is for Fendermint to recognise transactions coming from validators
            // and always put them into the block to facilitate checkpointing.
            gas_fee_cap: self.gas_fee_cap.clone(),
            gas_premium: self.gas_premium.clone(),
        };

        let gas_estimate = self
            .client
            .estimate_gas(message.clone(), FvmQueryHeight::Committed)
            .await
            .context("failed to estimate broadcaster gas")?;

        if gas_estimate.value.exit_code.is_success() {
            message.gas_limit = gas_estimate.value.gas_limit;
        } else {
            bail!(
                "failed to estimate gas: {} - {}",
                gas_estimate.value.exit_code,
                gas_estimate.value.info
            );
        }

        message.sequence = self
            .sequence()
            .await
            .context("failed to get broadcaster sequence")?;

        let message = SignedMessage::new_secp256k1(message, &self.secret_key, chain_id)?;
        let _message = ChainMessage::Signed(message);

        todo!()
    }

    /// Fetch the current nonce to be used in the next message.
    async fn sequence(&self) -> anyhow::Result<u64> {
        let res = self
            .client
            .actor_state(&self.addr, FvmQueryHeight::Pending)
            .await
            .context("failed to get broadcaster actor state")?;

        match res.value {
            Some((_, state)) => Ok(state.sequence),
            None => Err(anyhow!("broadcaster actor {} cannot be found", self.addr)),
        }
    }
}
