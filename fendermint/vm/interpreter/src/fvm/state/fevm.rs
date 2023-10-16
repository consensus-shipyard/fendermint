// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::any::type_name;
use std::fmt::Debug;
use std::{marker::PhantomData, sync::Arc};

use anyhow::{anyhow, bail, Context};
use ethers::abi::{AbiDecode, AbiEncode, Detokenize};
use ethers::core::types as et;
use ethers::prelude::{decode_function_data, ContractRevert};
use ethers::providers as ep;
use fendermint_vm_actor_interface::{eam::EthAddress, evm, system};
use fendermint_vm_message::conv::from_eth;
use fvm::executor::ApplyFailure;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{BytesDe, BytesSer, RawBytes};
use fvm_shared::{address::Address, econ::TokenAmount, error::ExitCode, message::Message};

use super::FvmExecState;

pub type MockProvider = ep::Provider<ep::MockProvider>;
pub type MockContractCall<T> = ethers::prelude::ContractCall<MockProvider, T>;

/// Result of trying to decode the data returned in failures as reverts.
///
/// The `E` type is supposed to be the enum unifying all errors that the contract can emit.
pub enum CallError<E> {
    /// The contract reverted with one of the expected custom errors.
    Revert(E),
    /// Some other error occurred that we could not decode.
    Raw(Vec<u8>),
}

impl<E> std::fmt::Debug for CallError<E>
where
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Revert(e) => write!(f, "{}:{:?}", type_name::<E>(), e),
            CallError::Raw(bz) if bz.is_empty() => write!(f, "<no data; potential ABI mismatch>"),
            CallError::Raw(bz) => write!(f, "0x{}", hex::encode(bz)),
        }
    }
}

/// Type we can use if a contract does not return revert errors, e.g. because it's all read-only views.
#[derive(Clone)]
pub struct NoRevert;

impl ContractRevert for NoRevert {
    fn valid_selector(_selector: et::Selector) -> bool {
        false
    }
}
impl AbiDecode for NoRevert {
    fn decode(_bytes: impl AsRef<[u8]>) -> Result<Self, ethers::contract::AbiError> {
        unimplemented!("selector doesn't match anything")
    }
}
impl AbiEncode for NoRevert {
    fn encode(self) -> Vec<u8> {
        unimplemented!("selector doesn't match anything")
    }
}

impl std::fmt::Debug for NoRevert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "contract not expected to revert")
    }
}

/// Facilitate calling FEVM contracts through their Ethers ABI bindings by
/// 1. serializing parameters,
/// 2. sending a message to the FVM, and
/// 3. deserializing the return value
///
/// Example:
/// ```no_run
/// use fendermint_vm_actor_interface::{eam::EthAddress, ipc::GATEWAY_ACTOR_ID};
/// use ipc_actors_abis::gateway_getter_facet::GatewayGetterFacet;
/// # use fendermint_vm_interpreter::fvm::state::fevm::ContractCaller;
/// # use fendermint_vm_interpreter::fvm::state::FvmExecState;
/// # use fendermint_vm_interpreter::fvm::store::memory::MemoryBlockstore as DB;
///
/// let caller = ContractCaller::new(
///     EthAddress::from_id(GATEWAY_ACTOR_ID),
///     GatewayGetterFacet::new
/// );
///
/// let mut state: FvmExecState<DB> = todo!();
///
/// let _period: u64 = caller.call(&mut state, |c| c.bottom_up_check_period()).unwrap();
/// ```
#[derive(Clone)]
pub struct ContractCaller<DB, C, E> {
    addr: Address,
    contract: C,
    store: PhantomData<DB>,
    error: PhantomData<E>,
}

impl<DB, C, E> ContractCaller<DB, C, E> {
    /// Create a new contract caller with the contract's Ethereum address and ABI bindings:
    pub fn new<F>(addr: EthAddress, contract: F) -> Self
    where
        F: FnOnce(et::Address, Arc<MockProvider>) -> C,
    {
        let (client, _mock) = ep::Provider::mocked();
        let contract = contract(addr.into(), std::sync::Arc::new(client));
        Self {
            addr: Address::from(addr),
            contract,
            store: PhantomData,
            error: PhantomData,
        }
    }

    /// Get a reference to the wrapped contract to construct messages without callign anything.
    pub fn contract(&self) -> &C {
        &self.contract
    }
}

impl<DB, C, E> ContractCaller<DB, C, E>
where
    DB: Blockstore,
    E: ContractRevert + Debug,
{
    /// Call an EVM method implicitly to read its return value.
    ///
    /// Returns an error if the return code shows is not successful;
    /// intended to be used with methods that are expected succeed.
    pub fn call<T, F>(&self, state: &mut FvmExecState<DB>, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&C) -> MockContractCall<T>,
        T: Detokenize,
    {
        match self.try_call(state, f)? {
            Ok(value) => Ok(value),
            Err((exit_code, failure_info, error)) => {
                bail!(
                    "failed to execute contract call to {}:\ncode: {}\nerror: {:?}\ninfo: {}",
                    self.addr,
                    exit_code.value(),
                    error,
                    failure_info.map(|i| i.to_string()).unwrap_or_default(),
                );
            }
        }
    }

    /// Call an EVM method implicitly to read its return value.
    ///
    /// Returns either the result or the exit code if it's not successful;
    /// intended to be used with methods that are expected to fail under certain conditions.
    pub fn try_call<T, F>(
        &self,
        state: &mut FvmExecState<DB>,
        f: F,
    ) -> anyhow::Result<Result<T, (ExitCode, Option<ApplyFailure>, CallError<E>)>>
    where
        F: FnOnce(&C) -> MockContractCall<T>,
        T: Detokenize,
    {
        let call = f(&self.contract);
        let calldata = call.calldata().ok_or_else(|| anyhow!("missing calldata"))?;
        let calldata = RawBytes::serialize(BytesSer(&calldata))?;

        let from = call
            .tx
            .from()
            .map(|addr| Address::from(EthAddress::from(*addr)))
            .unwrap_or(system::SYSTEM_ACTOR_ADDR);

        let value = call
            .tx
            .value()
            .map(from_eth::to_fvm_tokens)
            .unwrap_or_else(|| TokenAmount::from_atto(0));

        // We send off a read-only query to an EVM actor at the given address.
        let msg = Message {
            version: Default::default(),
            from,
            to: self.addr,
            sequence: 0,
            value,
            method_num: evm::Method::InvokeContract as u64,
            params: calldata,
            gas_limit: fvm_shared::BLOCK_GAS_LIMIT,
            gas_fee_cap: TokenAmount::from_atto(0),
            gas_premium: TokenAmount::from_atto(0),
        };

        let (ret, _) = state.execute_implicit(msg).context("failed to call FEVM")?;

        if !ret.msg_receipt.exit_code.is_success() {
            // The EVM actor might return some data in the output.
            let output = ret
                .msg_receipt
                .return_data
                .deserialize::<BytesDe>()
                .map(|bz| bz.0)
                .context("failed to deserialize error data")?;

            let error = match decode_revert::<E>(&output) {
                Some(e) => CallError::Revert(e),
                None => CallError::Raw(output),
            };

            Ok(Err((ret.msg_receipt.exit_code, ret.failure_info, error)))
        } else {
            let data = ret
                .msg_receipt
                .return_data
                .deserialize::<BytesDe>()
                .context("failed to deserialize return data")?;

            let value = decode_function_data(&call.function, data.0, false)
                .context("failed to decode bytes")?;

            Ok(Ok(value))
        }
    }
}

/// Fixed decoding until https://github.com/gakonst/ethers-rs/pull/2637 is released.
fn decode_revert<E: ContractRevert>(data: &[u8]) -> Option<E> {
    E::decode_with_selector(data).or_else(|| {
        if data.len() < 4 {
            return None;
        }
        // There is a bug fixed by the above PR that chops the selector off.
        // By doubling it up, after chopping off it should still be present.
        let double_prefix = [&data[..4], data].concat();
        E::decode_with_selector(&double_prefix)
    })
}

#[cfg(test)]
mod tests {
    use ethers::{contract::ContractRevert, types::Bytes};
    use ipc_actors_abis::gateway_manager_facet::{GatewayManagerFacetErrors, InsufficientFunds};

    use crate::fvm::state::fevm::decode_revert;

    #[test]
    fn decode_custom_error() {
        // An example of binary data corresponding to `InsufficientFunds`
        let bz: Bytes = "0x356680b7".parse().unwrap();

        let selector = bz[..4].try_into().expect("it's 4 bytes");

        assert!(
            GatewayManagerFacetErrors::valid_selector(selector),
            "it should be a valid selector"
        );

        let err =
            decode_revert::<GatewayManagerFacetErrors>(&bz).expect("could not decode as revert");

        assert_eq!(
            err,
            GatewayManagerFacetErrors::InsufficientFunds(InsufficientFunds)
        )
    }
}
