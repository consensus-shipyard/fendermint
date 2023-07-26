// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

// The IPC actors have bindings in `fendermint_vm_ipc_actors`.
// Here we define stable IDs for them, so we can deploy the
// Solidity contracts during genesis.

define_id!(GATEWAY { id: 20 });
define_id!(SUBNET_REGISTRY { id: 21 });

pub mod gateway {
    use ethers::contract::{EthAbiCodec, EthAbiType};
    use ethers::core::types::U256;
    use fendermint_vm_ipc_actors::gateway::SubnetID;

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

    #[cfg(test)]
    mod tests {
        use ethers::core::types::U256;
        use ethers_core::abi::Tokenize;
        use fendermint_vm_ipc_actors::gateway::SubnetID;

        use super::ConstructorParameters;

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

            let tokens = cp.into_tokens();

            let constructor = fendermint_vm_ipc_actors::gateway::GATEWAY_ABI
                .constructor()
                .expect("Gatway has a constructor");

            constructor
                .encode_input(vec![], &tokens)
                .expect("should encode constructor input");
        }
    }
}

pub mod subnet_registry {}
