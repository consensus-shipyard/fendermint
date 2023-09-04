// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! Handles the type conversion to ethers contract types

use crate::IPCParentFinality;
use anyhow::anyhow;
use ethers::abi::Token;
use ethers::types::Bytes;
use ethers::types::U256;
use fendermint_vm_ipc_actors::{gateway_getter_facet, gateway_router_facet};
use fvm_shared::address::{Address, Payload};
use fvm_shared::clock::ChainEpoch;
use ipc_agent_sdk::message::ipc::ValidatorSet;
use std::marker::PhantomData;
use std::str::FromStr;

const COMMIT_PARENT_FINALITY_FUNC_NAME: &str = "commitParentFinality";
const GET_PARENT_FINALITY_FUNC_NAME: &str = "getParentFinality";

impl From<IPCParentFinality> for gateway_router_facet::ParentFinality {
    fn from(value: IPCParentFinality) -> Self {
        let mut array = [0u8; 32];
        array.copy_from_slice(&value.block_hash[0..32]);

        Self {
            height: U256::from(value.height),
            block_hash: array,
        }
    }
}

impl From<gateway_getter_facet::ParentFinality> for IPCParentFinality {
    fn from(value: gateway_getter_facet::ParentFinality) -> Self {
        IPCParentFinality {
            height: value.height.as_u64(),
            block_hash: value.block_hash.to_vec(),
        }
    }
}

/// Converts a Rust type FVM address into its underlying payload
/// so it can be represented internally in a Solidity contract.
fn addr_payload_to_bytes(payload: Payload) -> Bytes {
    match payload {
        Payload::Secp256k1(v) => ethers::types::Bytes::from(v),
        Payload::Delegated(d) => {
            let addr = d.subaddress();
            let b = ethers::abi::encode(&[Token::Tuple(vec![
                Token::Uint(U256::from(d.namespace())),
                Token::Uint(U256::from(addr.len())),
                Token::Bytes(addr.to_vec()),
            ])]);
            ethers::types::Bytes::from(b)
        }
        _ => unimplemented!(),
    }
}

fn convert_addr(addr: Address) -> gateway_router_facet::FvmAddress {
    gateway_router_facet::FvmAddress {
        addr_type: addr.protocol() as u8,
        payload: addr_payload_to_bytes(addr.into_payload()),
    }
}

/// The decode function call return value in solidity
pub struct DecodeFunctionReturn<T> {
    _phantom: PhantomData<T>,
}

/// The equivalent of abi.encodeWithSignature in solidity
pub struct EncodeWithSignature<T> {
    _phantom: PhantomData<T>,
}

impl EncodeWithSignature<gateway_router_facet::CommitParentFinalityCall> {
    pub fn encode(
        finality: IPCParentFinality,
        validator_set: ValidatorSet,
    ) -> anyhow::Result<Vec<u8>> {
        let commit_function = gateway_router_facet::GATEWAYROUTERFACET_ABI
            .functions
            .get(COMMIT_PARENT_FINALITY_FUNC_NAME)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi function map does not have {}",
                    COMMIT_PARENT_FINALITY_FUNC_NAME
                )
            })?
            .get(0)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi vec does not have {}",
                    COMMIT_PARENT_FINALITY_FUNC_NAME
                )
            })?;

        let validators = validator_set.validators.unwrap_or_default();

        let mut addresses = vec![];
        let mut weights = vec![];
        for validator in validators {
            let raw_address = validator.worker_addr.unwrap_or(validator.addr);
            let addr = Address::from_str(&raw_address)?;
            addresses.push(convert_addr(addr));
            weights.push(U256::from_dec_str(&validator.weight)?);
        }

        let data = ethers::contract::encode_function_data(
            commit_function,
            gateway_router_facet::CommitParentFinalityCall {
                finality: gateway_router_facet::ParentFinality::try_from(finality)?,
                validators: addresses,
                weights,
            },
        )?;

        Ok(data.to_vec())
    }
}

impl EncodeWithSignature<gateway_getter_facet::GetParentFinalityCall> {
    pub fn encode(height: ChainEpoch) -> anyhow::Result<Vec<u8>> {
        let function = gateway_getter_facet::GATEWAYGETTERFACET_ABI
            .functions
            .get(GET_PARENT_FINALITY_FUNC_NAME)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi function map does not have {}",
                    GET_PARENT_FINALITY_FUNC_NAME
                )
            })?
            .get(0)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi vec does not have {}",
                    GET_PARENT_FINALITY_FUNC_NAME
                )
            })?;

        let data = ethers::contract::encode_function_data(function, U256::from(height))?;

        Ok(data.to_vec())
    }
}

impl DecodeFunctionReturn<IPCParentFinality> {
    pub fn decode(bytes: Vec<u8>) -> anyhow::Result<IPCParentFinality> {
        let function = gateway_getter_facet::GATEWAYGETTERFACET_ABI
            .functions
            .get(GET_PARENT_FINALITY_FUNC_NAME)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi function map does not have {}",
                    GET_PARENT_FINALITY_FUNC_NAME
                )
            })?
            .get(0)
            .ok_or_else(|| {
                anyhow!(
                    "report bug, abi vec does not have {}",
                    GET_PARENT_FINALITY_FUNC_NAME
                )
            })?;

        let finality = ethers::contract::decode_function_data::<
            gateway_getter_facet::ParentFinality,
            _,
        >(function, bytes, false)?;
        Ok(IPCParentFinality::from(finality))
    }
}
