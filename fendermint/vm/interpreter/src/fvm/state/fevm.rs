// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::sync::Arc;

use anyhow::{anyhow, bail, Context};
use ethers::abi::Detokenize;
use ethers::core::types as et;
use ethers::prelude::decode_function_data;
use ethers::providers as ep;
use fendermint_vm_actor_interface::{eam::EthAddress, evm, system};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{BytesDe, BytesSer, RawBytes};
use fvm_shared::{address::Address, econ::TokenAmount, message::Message};

use super::FvmExecState;

type MockProvider = ep::Provider<ep::MockProvider>;
type MockContractCall<T> = ethers::prelude::ContractCall<MockProvider, T>;

/// Facilitate calling FEVM contracts through their Ethers ABI bindings by
/// 1. serializing parameters,
/// 2. sending a message to the FVM, and
/// 3. deserializing the return value
pub struct ContractCaller<C> {
    addr: Address,
    contract: C,
}

impl<C> ContractCaller<C> {
    pub fn new<F>(addr: EthAddress, contract: F) -> Self
    where
        F: FnOnce(et::Address, Arc<MockProvider>) -> C,
    {
        let (client, _mock) = ep::Provider::mocked();
        let contract = contract(
            et::Address::from_slice(&addr.0),
            std::sync::Arc::new(client),
        );
        Self {
            addr: Address::from(addr),
            contract,
        }
    }

    /// Call a read-only EVM method.
    ///
    /// Returns an error if the return code shows is not successful,
    /// intended to be used with methods that are expected succeed.
    pub fn call<T, F, DB>(&self, state: &mut FvmExecState<DB>, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&C) -> MockContractCall<T>,
        T: Detokenize,
        DB: Blockstore,
    {
        let call = f(&self.contract);
        let calldata = call.calldata().ok_or_else(|| anyhow!("missing calldata"))?;
        let calldata = RawBytes::serialize(BytesSer(&calldata))?;

        // We send off a read-only query to an EVM actor at the given address.
        let msg = Message {
            version: Default::default(),
            from: system::SYSTEM_ACTOR_ADDR,
            to: self.addr,
            sequence: 0,
            value: TokenAmount::from_atto(0),
            method_num: evm::Method::InvokeContract as u64,
            params: calldata,
            gas_limit: fvm_shared::BLOCK_GAS_LIMIT,
            gas_fee_cap: TokenAmount::from_atto(0),
            gas_premium: TokenAmount::from_atto(0),
        };

        let (ret, _) = state.execute_implicit(msg)?;

        if !ret.msg_receipt.exit_code.is_success() {
            bail!(
                "failed to execute contract call to {}: {} - {}",
                self.addr,
                ret.msg_receipt.exit_code.value(),
                ret.failure_info.map(|i| i.to_string()).unwrap_or_default()
            );
        }

        let data = ret
            .msg_receipt
            .return_data
            .deserialize::<BytesDe>()
            .context("failed to deserialize return data")?;

        let value = decode_function_data(&call.function, data.0, false)
            .context("failed to decode bytes")?;

        Ok(value)
    }
}
