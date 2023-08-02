// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// The IPC actors have bindings in `fendermint_vm_ipc_actors`.
// Here we define stable IDs for them, so we can deploy the
// Solidity contracts during genesis.

define_id!(GATEWAY { id: 64 });
define_id!(SUBNETREGISTRY { id: 65 });

pub use fendermint_vm_ipc_actors::gateway_manager_facet::SubnetID;

pub mod gateway {
    use super::SubnetID;
    use ethers::contract::{EthAbiCodec, EthAbiType};
    use ethers::core::types::{H160, U256};
    use fendermint_vm_genesis::ipc::GatewayParams;
    use fvm_shared::address::Payload;
    use fvm_shared::econ::TokenAmount;

    use crate::eam::{self, EthAddress};

    // Constructor parameters aren't generated as part of the Rust bindings.

    /// Container type `ConstructorParameters`.
    ///
    /// See [Gateway.sol](https://github.com/consensus-shipyard/ipc-solidity-actors/blob/v0.1.0/src/Gateway.sol#L176)
    #[derive(Clone, EthAbiType, EthAbiCodec, Default, Debug, PartialEq, Eq, Hash)]
    pub struct ConstructorParameters {
        pub network_name: SubnetID,
        pub bottom_up_check_period: u64,
        pub top_down_check_period: u64,
        pub msg_fee: U256,
        pub majority_percentage: u8,
    }

    impl TryFrom<GatewayParams> for ConstructorParameters {
        type Error = fvm_shared::address::Error;

        fn try_from(value: GatewayParams) -> Result<Self, Self::Error> {
            let mut route = Vec::new();
            for addr in value.subnet_id.children() {
                let addr = match addr.payload() {
                    Payload::ID(id) => EthAddress::from_id(*id),
                    Payload::Delegated(da)
                        if da.namespace() == eam::EAM_ACTOR_ID && da.subaddress().len() == 20 =>
                    {
                        EthAddress(da.subaddress().try_into().expect("checked length"))
                    }
                    _ => return Err(fvm_shared::address::Error::InvalidPayload),
                };
                route.push(H160::from(addr.0))
            }
            Ok(Self {
                network_name: SubnetID {
                    root: value.subnet_id.root_id(),
                    route,
                },
                bottom_up_check_period: value.bottom_up_check_period,
                top_down_check_period: value.top_down_check_period,
                msg_fee: tokens_to_u256(value.msg_fee),
                majority_percentage: value.majority_percentage,
            })
        }
    }

    fn tokens_to_u256(value: TokenAmount) -> U256 {
        // XXX: Ignoring any error resulting from larger fee than what fits into U256. This is in genesis after all.
        U256::from_big_endian(&value.atto().to_bytes_be().1)
    }

    #[cfg(test)]
    mod tests {
        use ethers::core::types::U256;
        use ethers_core::abi::Tokenize;
        use fvm_shared::{bigint::BigInt, econ::TokenAmount};
        use std::str::FromStr;

        use crate::ipc::tests::{check_param_types, constructor_param_types};

        use super::{tokens_to_u256, ConstructorParameters, SubnetID};

        #[test]
        fn tokenize_constructor_params() {
            let cp = ConstructorParameters {
                network_name: SubnetID {
                    root: 0,
                    route: Vec::new(),
                },
                bottom_up_check_period: 100,
                top_down_check_period: 100,
                msg_fee: U256::from(0),
                majority_percentage: 67,
            };

            // It looks like if we pass just the record then it will be passed as 5 tokens,
            // but the constructor only expects one parameter, and it has to be a tuple.
            let cp = (cp,);

            let tokens = cp.into_tokens();

            let cons = fendermint_vm_ipc_actors::gateway_diamond::GATEWAYDIAMOND_ABI
                .constructor()
                .expect("Gateway has a constructor");

            let param_types = constructor_param_types(cons);

            check_param_types(&tokens, &param_types).unwrap();

            cons.encode_input(vec![], &tokens)
                .expect("should encode constructor input");
        }

        #[test]
        #[should_panic]
        fn max_fee_exceeded() {
            let mut value = BigInt::from_str(&U256::MAX.to_string()).unwrap();
            value += 1;
            let value = TokenAmount::from_atto(value);
            let _ = tokens_to_u256(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use ethers_core::abi::{Constructor, ParamType, Token};

    /// Check all tokens against expected parameters; return any offending one.
    ///
    /// Based on [Tokens::types_check]
    pub fn check_param_types(tokens: &[Token], param_types: &[ParamType]) -> anyhow::Result<()> {
        if param_types.len() != tokens.len() {
            bail!(
                "different number of parameters; expected {}, got {}",
                param_types.len(),
                tokens.len()
            );
        }

        for (i, (pt, t)) in param_types.iter().zip(tokens).enumerate() {
            if !t.type_check(pt) {
                bail!("parameter {i} didn't type check: expected {pt:?}, got {t:?}");
            }
        }

        Ok(())
    }

    /// Returns all input params of given constructor.
    ///
    /// Based on [Constructor::param_types]
    pub fn constructor_param_types(cons: &Constructor) -> Vec<ParamType> {
        cons.inputs.iter().map(|p| p.kind.clone()).collect()
    }
}
