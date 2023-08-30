pub use gateway_manager_facet::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types,
)]
pub mod gateway_manager_facet {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("addStake"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("addStake"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("fund"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("fund"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("subnetId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Address,
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct SubnetID"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FvmAddress"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("initGenesisEpoch"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("initGenesisEpoch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("genesisEpoch"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("kill"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("kill"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("register"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("register"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("release"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("release"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FvmAddress"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("releaseRewards"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("releaseRewards"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("releaseStake"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("releaseStake"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setMembership"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setMembership"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("validators"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address[]"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("weights"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256[]"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
            ]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("AddressEmptyCode"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("AddressEmptyCode"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("target"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("AddressInsufficientBalance"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "AddressInsufficientBalance",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("account"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("AlreadyInitialized"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("AlreadyInitialized"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("AlreadyRegisteredSubnet"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "AlreadyRegisteredSubnet",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("CallFailed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("CallFailed"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("CannotReleaseZero"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("CannotReleaseZero"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("FailedInnerCall"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("FailedInnerCall"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InsufficientFunds"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("InsufficientFunds"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidActorAddress"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidActorAddress",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotEmptySubnetCircSupply"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "NotEmptySubnetCircSupply",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotEnoughFee"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotEnoughFee"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotEnoughFunds"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotEnoughFunds"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotEnoughFundsToRelease"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "NotEnoughFundsToRelease",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotRegisteredSubnet"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "NotRegisteredSubnet",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotSystemActor"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotSystemActor"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ReentrancyError"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("ReentrancyError"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorWeightIsZero"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ValidatorWeightIsZero",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned(
                        "ValidatorsAndWeightsLengthMismatch",
                    ),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ValidatorsAndWeightsLengthMismatch",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static GATEWAYMANAGERFACET_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    pub struct GatewayManagerFacet<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for GatewayManagerFacet<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for GatewayManagerFacet<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for GatewayManagerFacet<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for GatewayManagerFacet<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(GatewayManagerFacet))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> GatewayManagerFacet<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    GATEWAYMANAGERFACET_ABI.clone(),
                    client,
                ),
            )
        }
        ///Calls the contract's `addStake` (0x5a627dbc) function
        pub fn add_stake(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([90, 98, 125, 188], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `fund` (0x18f44b70) function
        pub fn fund(
            &self,
            subnet_id: SubnetID,
            to: FvmAddress,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([24, 244, 75, 112], (subnet_id, to))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `initGenesisEpoch` (0x13f35388) function
        pub fn init_genesis_epoch(
            &self,
            genesis_epoch: u64,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([19, 243, 83, 136], genesis_epoch)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `kill` (0x41c0e1b5) function
        pub fn kill(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([65, 192, 225, 181], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `register` (0x1aa3a008) function
        pub fn register(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([26, 163, 160, 8], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `release` (0x6b2c1eef) function
        pub fn release(
            &self,
            to: FvmAddress,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([107, 44, 30, 239], (to,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `releaseRewards` (0xf8703bb8) function
        pub fn release_rewards(
            &self,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([248, 112, 59, 184], amount)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `releaseStake` (0x45f54485) function
        pub fn release_stake(
            &self,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([69, 245, 68, 133], amount)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setMembership` (0xf75bc557) function
        pub fn set_membership(
            &self,
            validators: ::std::vec::Vec<::ethers::core::types::Address>,
            weights: ::std::vec::Vec<::ethers::core::types::U256>,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([247, 91, 197, 87], (validators, weights))
                .expect("method not found (this should never happen)")
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for GatewayManagerFacet<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `AddressEmptyCode` with signature `AddressEmptyCode(address)` and selector `0x9996b315`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "AddressEmptyCode", abi = "AddressEmptyCode(address)")]
    pub struct AddressEmptyCode {
        pub target: ::ethers::core::types::Address,
    }
    ///Custom Error type `AddressInsufficientBalance` with signature `AddressInsufficientBalance(address)` and selector `0xcd786059`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(
        name = "AddressInsufficientBalance",
        abi = "AddressInsufficientBalance(address)"
    )]
    pub struct AddressInsufficientBalance {
        pub account: ::ethers::core::types::Address,
    }
    ///Custom Error type `AlreadyInitialized` with signature `AlreadyInitialized()` and selector `0x0dc149f0`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "AlreadyInitialized", abi = "AlreadyInitialized()")]
    pub struct AlreadyInitialized;
    ///Custom Error type `AlreadyRegisteredSubnet` with signature `AlreadyRegisteredSubnet()` and selector `0x36a719be`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "AlreadyRegisteredSubnet", abi = "AlreadyRegisteredSubnet()")]
    pub struct AlreadyRegisteredSubnet;
    ///Custom Error type `CallFailed` with signature `CallFailed()` and selector `0x3204506f`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "CallFailed", abi = "CallFailed()")]
    pub struct CallFailed;
    ///Custom Error type `CannotReleaseZero` with signature `CannotReleaseZero()` and selector `0xc79cad7b`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "CannotReleaseZero", abi = "CannotReleaseZero()")]
    pub struct CannotReleaseZero;
    ///Custom Error type `FailedInnerCall` with signature `FailedInnerCall()` and selector `0x1425ea42`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "FailedInnerCall", abi = "FailedInnerCall()")]
    pub struct FailedInnerCall;
    ///Custom Error type `InsufficientFunds` with signature `InsufficientFunds()` and selector `0x356680b7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "InsufficientFunds", abi = "InsufficientFunds()")]
    pub struct InsufficientFunds;
    ///Custom Error type `InvalidActorAddress` with signature `InvalidActorAddress()` and selector `0x70e45109`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "InvalidActorAddress", abi = "InvalidActorAddress()")]
    pub struct InvalidActorAddress;
    ///Custom Error type `NotEmptySubnetCircSupply` with signature `NotEmptySubnetCircSupply()` and selector `0xf8cf8e02`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotEmptySubnetCircSupply", abi = "NotEmptySubnetCircSupply()")]
    pub struct NotEmptySubnetCircSupply;
    ///Custom Error type `NotEnoughFee` with signature `NotEnoughFee()` and selector `0x688e55ae`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotEnoughFee", abi = "NotEnoughFee()")]
    pub struct NotEnoughFee;
    ///Custom Error type `NotEnoughFunds` with signature `NotEnoughFunds()` and selector `0x81b5ad68`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotEnoughFunds", abi = "NotEnoughFunds()")]
    pub struct NotEnoughFunds;
    ///Custom Error type `NotEnoughFundsToRelease` with signature `NotEnoughFundsToRelease()` and selector `0x79b33e79`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotEnoughFundsToRelease", abi = "NotEnoughFundsToRelease()")]
    pub struct NotEnoughFundsToRelease;
    ///Custom Error type `NotRegisteredSubnet` with signature `NotRegisteredSubnet()` and selector `0xe991abd0`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotRegisteredSubnet", abi = "NotRegisteredSubnet()")]
    pub struct NotRegisteredSubnet;
    ///Custom Error type `NotSystemActor` with signature `NotSystemActor()` and selector `0xf0d97f3b`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotSystemActor", abi = "NotSystemActor()")]
    pub struct NotSystemActor;
    ///Custom Error type `ReentrancyError` with signature `ReentrancyError()` and selector `0x29f745a7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "ReentrancyError", abi = "ReentrancyError()")]
    pub struct ReentrancyError;
    ///Custom Error type `ValidatorWeightIsZero` with signature `ValidatorWeightIsZero()` and selector `0x389b457d`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "ValidatorWeightIsZero", abi = "ValidatorWeightIsZero()")]
    pub struct ValidatorWeightIsZero;
    ///Custom Error type `ValidatorsAndWeightsLengthMismatch` with signature `ValidatorsAndWeightsLengthMismatch()` and selector `0x465f0a7d`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(
        name = "ValidatorsAndWeightsLengthMismatch",
        abi = "ValidatorsAndWeightsLengthMismatch()"
    )]
    pub struct ValidatorsAndWeightsLengthMismatch;
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum GatewayManagerFacetErrors {
        AddressEmptyCode(AddressEmptyCode),
        AddressInsufficientBalance(AddressInsufficientBalance),
        AlreadyInitialized(AlreadyInitialized),
        AlreadyRegisteredSubnet(AlreadyRegisteredSubnet),
        CallFailed(CallFailed),
        CannotReleaseZero(CannotReleaseZero),
        FailedInnerCall(FailedInnerCall),
        InsufficientFunds(InsufficientFunds),
        InvalidActorAddress(InvalidActorAddress),
        NotEmptySubnetCircSupply(NotEmptySubnetCircSupply),
        NotEnoughFee(NotEnoughFee),
        NotEnoughFunds(NotEnoughFunds),
        NotEnoughFundsToRelease(NotEnoughFundsToRelease),
        NotRegisteredSubnet(NotRegisteredSubnet),
        NotSystemActor(NotSystemActor),
        ReentrancyError(ReentrancyError),
        ValidatorWeightIsZero(ValidatorWeightIsZero),
        ValidatorsAndWeightsLengthMismatch(ValidatorsAndWeightsLengthMismatch),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for GatewayManagerFacetErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded)
                = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded)
                = <AddressEmptyCode as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::AddressEmptyCode(decoded));
            }
            if let Ok(decoded)
                = <AddressInsufficientBalance as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::AddressInsufficientBalance(decoded));
            }
            if let Ok(decoded)
                = <AlreadyInitialized as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::AlreadyInitialized(decoded));
            }
            if let Ok(decoded)
                = <AlreadyRegisteredSubnet as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::AlreadyRegisteredSubnet(decoded));
            }
            if let Ok(decoded)
                = <CallFailed as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::CallFailed(decoded));
            }
            if let Ok(decoded)
                = <CannotReleaseZero as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::CannotReleaseZero(decoded));
            }
            if let Ok(decoded)
                = <FailedInnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::FailedInnerCall(decoded));
            }
            if let Ok(decoded)
                = <InsufficientFunds as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::InsufficientFunds(decoded));
            }
            if let Ok(decoded)
                = <InvalidActorAddress as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::InvalidActorAddress(decoded));
            }
            if let Ok(decoded)
                = <NotEmptySubnetCircSupply as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::NotEmptySubnetCircSupply(decoded));
            }
            if let Ok(decoded)
                = <NotEnoughFee as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotEnoughFee(decoded));
            }
            if let Ok(decoded)
                = <NotEnoughFunds as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotEnoughFunds(decoded));
            }
            if let Ok(decoded)
                = <NotEnoughFundsToRelease as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::NotEnoughFundsToRelease(decoded));
            }
            if let Ok(decoded)
                = <NotRegisteredSubnet as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotRegisteredSubnet(decoded));
            }
            if let Ok(decoded)
                = <NotSystemActor as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotSystemActor(decoded));
            }
            if let Ok(decoded)
                = <ReentrancyError as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::ReentrancyError(decoded));
            }
            if let Ok(decoded)
                = <ValidatorWeightIsZero as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::ValidatorWeightIsZero(decoded));
            }
            if let Ok(decoded)
                = <ValidatorsAndWeightsLengthMismatch as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::ValidatorsAndWeightsLengthMismatch(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for GatewayManagerFacetErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::AddressEmptyCode(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::AddressInsufficientBalance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::AlreadyInitialized(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::AlreadyRegisteredSubnet(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CallFailed(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CannotReleaseZero(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::FailedInnerCall(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InsufficientFunds(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidActorAddress(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEmptySubnetCircSupply(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEnoughFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEnoughFunds(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEnoughFundsToRelease(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotRegisteredSubnet(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotSystemActor(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ReentrancyError(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorWeightIsZero(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorsAndWeightsLengthMismatch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for GatewayManagerFacetErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <AddressEmptyCode as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <AddressInsufficientBalance as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <AlreadyInitialized as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <AlreadyRegisteredSubnet as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <CallFailed as ::ethers::contract::EthError>::selector() => true,
                _ if selector
                    == <CannotReleaseZero as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <FailedInnerCall as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InsufficientFunds as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidActorAddress as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotEmptySubnetCircSupply as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotEnoughFee as ::ethers::contract::EthError>::selector() => true,
                _ if selector
                    == <NotEnoughFunds as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotEnoughFundsToRelease as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotRegisteredSubnet as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotSystemActor as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ReentrancyError as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ValidatorWeightIsZero as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ValidatorsAndWeightsLengthMismatch as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for GatewayManagerFacetErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AddressEmptyCode(element) => ::core::fmt::Display::fmt(element, f),
                Self::AddressInsufficientBalance(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::AlreadyInitialized(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::AlreadyRegisteredSubnet(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CallFailed(element) => ::core::fmt::Display::fmt(element, f),
                Self::CannotReleaseZero(element) => ::core::fmt::Display::fmt(element, f),
                Self::FailedInnerCall(element) => ::core::fmt::Display::fmt(element, f),
                Self::InsufficientFunds(element) => ::core::fmt::Display::fmt(element, f),
                Self::InvalidActorAddress(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotEmptySubnetCircSupply(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotEnoughFee(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotEnoughFunds(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotEnoughFundsToRelease(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotRegisteredSubnet(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotSystemActor(element) => ::core::fmt::Display::fmt(element, f),
                Self::ReentrancyError(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorWeightIsZero(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ValidatorsAndWeightsLengthMismatch(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for GatewayManagerFacetErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<AddressEmptyCode> for GatewayManagerFacetErrors {
        fn from(value: AddressEmptyCode) -> Self {
            Self::AddressEmptyCode(value)
        }
    }
    impl ::core::convert::From<AddressInsufficientBalance>
    for GatewayManagerFacetErrors {
        fn from(value: AddressInsufficientBalance) -> Self {
            Self::AddressInsufficientBalance(value)
        }
    }
    impl ::core::convert::From<AlreadyInitialized> for GatewayManagerFacetErrors {
        fn from(value: AlreadyInitialized) -> Self {
            Self::AlreadyInitialized(value)
        }
    }
    impl ::core::convert::From<AlreadyRegisteredSubnet> for GatewayManagerFacetErrors {
        fn from(value: AlreadyRegisteredSubnet) -> Self {
            Self::AlreadyRegisteredSubnet(value)
        }
    }
    impl ::core::convert::From<CallFailed> for GatewayManagerFacetErrors {
        fn from(value: CallFailed) -> Self {
            Self::CallFailed(value)
        }
    }
    impl ::core::convert::From<CannotReleaseZero> for GatewayManagerFacetErrors {
        fn from(value: CannotReleaseZero) -> Self {
            Self::CannotReleaseZero(value)
        }
    }
    impl ::core::convert::From<FailedInnerCall> for GatewayManagerFacetErrors {
        fn from(value: FailedInnerCall) -> Self {
            Self::FailedInnerCall(value)
        }
    }
    impl ::core::convert::From<InsufficientFunds> for GatewayManagerFacetErrors {
        fn from(value: InsufficientFunds) -> Self {
            Self::InsufficientFunds(value)
        }
    }
    impl ::core::convert::From<InvalidActorAddress> for GatewayManagerFacetErrors {
        fn from(value: InvalidActorAddress) -> Self {
            Self::InvalidActorAddress(value)
        }
    }
    impl ::core::convert::From<NotEmptySubnetCircSupply> for GatewayManagerFacetErrors {
        fn from(value: NotEmptySubnetCircSupply) -> Self {
            Self::NotEmptySubnetCircSupply(value)
        }
    }
    impl ::core::convert::From<NotEnoughFee> for GatewayManagerFacetErrors {
        fn from(value: NotEnoughFee) -> Self {
            Self::NotEnoughFee(value)
        }
    }
    impl ::core::convert::From<NotEnoughFunds> for GatewayManagerFacetErrors {
        fn from(value: NotEnoughFunds) -> Self {
            Self::NotEnoughFunds(value)
        }
    }
    impl ::core::convert::From<NotEnoughFundsToRelease> for GatewayManagerFacetErrors {
        fn from(value: NotEnoughFundsToRelease) -> Self {
            Self::NotEnoughFundsToRelease(value)
        }
    }
    impl ::core::convert::From<NotRegisteredSubnet> for GatewayManagerFacetErrors {
        fn from(value: NotRegisteredSubnet) -> Self {
            Self::NotRegisteredSubnet(value)
        }
    }
    impl ::core::convert::From<NotSystemActor> for GatewayManagerFacetErrors {
        fn from(value: NotSystemActor) -> Self {
            Self::NotSystemActor(value)
        }
    }
    impl ::core::convert::From<ReentrancyError> for GatewayManagerFacetErrors {
        fn from(value: ReentrancyError) -> Self {
            Self::ReentrancyError(value)
        }
    }
    impl ::core::convert::From<ValidatorWeightIsZero> for GatewayManagerFacetErrors {
        fn from(value: ValidatorWeightIsZero) -> Self {
            Self::ValidatorWeightIsZero(value)
        }
    }
    impl ::core::convert::From<ValidatorsAndWeightsLengthMismatch>
    for GatewayManagerFacetErrors {
        fn from(value: ValidatorsAndWeightsLengthMismatch) -> Self {
            Self::ValidatorsAndWeightsLengthMismatch(value)
        }
    }
    ///Container type for all input parameters for the `addStake` function with signature `addStake()` and selector `0x5a627dbc`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "addStake", abi = "addStake()")]
    pub struct AddStakeCall;
    ///Container type for all input parameters for the `fund` function with signature `fund((uint64,address[]),(uint8,bytes))` and selector `0x18f44b70`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "fund", abi = "fund((uint64,address[]),(uint8,bytes))")]
    pub struct FundCall {
        pub subnet_id: SubnetID,
        pub to: FvmAddress,
    }
    ///Container type for all input parameters for the `initGenesisEpoch` function with signature `initGenesisEpoch(uint64)` and selector `0x13f35388`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "initGenesisEpoch", abi = "initGenesisEpoch(uint64)")]
    pub struct InitGenesisEpochCall {
        pub genesis_epoch: u64,
    }
    ///Container type for all input parameters for the `kill` function with signature `kill()` and selector `0x41c0e1b5`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "kill", abi = "kill()")]
    pub struct KillCall;
    ///Container type for all input parameters for the `register` function with signature `register()` and selector `0x1aa3a008`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "register", abi = "register()")]
    pub struct RegisterCall;
    ///Container type for all input parameters for the `release` function with signature `release((uint8,bytes))` and selector `0x6b2c1eef`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "release", abi = "release((uint8,bytes))")]
    pub struct ReleaseCall {
        pub to: FvmAddress,
    }
    ///Container type for all input parameters for the `releaseRewards` function with signature `releaseRewards(uint256)` and selector `0xf8703bb8`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "releaseRewards", abi = "releaseRewards(uint256)")]
    pub struct ReleaseRewardsCall {
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `releaseStake` function with signature `releaseStake(uint256)` and selector `0x45f54485`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "releaseStake", abi = "releaseStake(uint256)")]
    pub struct ReleaseStakeCall {
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `setMembership` function with signature `setMembership(address[],uint256[])` and selector `0xf75bc557`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "setMembership", abi = "setMembership(address[],uint256[])")]
    pub struct SetMembershipCall {
        pub validators: ::std::vec::Vec<::ethers::core::types::Address>,
        pub weights: ::std::vec::Vec<::ethers::core::types::U256>,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum GatewayManagerFacetCalls {
        AddStake(AddStakeCall),
        Fund(FundCall),
        InitGenesisEpoch(InitGenesisEpochCall),
        Kill(KillCall),
        Register(RegisterCall),
        Release(ReleaseCall),
        ReleaseRewards(ReleaseRewardsCall),
        ReleaseStake(ReleaseStakeCall),
        SetMembership(SetMembershipCall),
    }
    impl ::ethers::core::abi::AbiDecode for GatewayManagerFacetCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded)
                = <AddStakeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::AddStake(decoded));
            }
            if let Ok(decoded)
                = <FundCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Fund(decoded));
            }
            if let Ok(decoded)
                = <InitGenesisEpochCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InitGenesisEpoch(decoded));
            }
            if let Ok(decoded)
                = <KillCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Kill(decoded));
            }
            if let Ok(decoded)
                = <RegisterCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Register(decoded));
            }
            if let Ok(decoded)
                = <ReleaseCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Release(decoded));
            }
            if let Ok(decoded)
                = <ReleaseRewardsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::ReleaseRewards(decoded));
            }
            if let Ok(decoded)
                = <ReleaseStakeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::ReleaseStake(decoded));
            }
            if let Ok(decoded)
                = <SetMembershipCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::SetMembership(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for GatewayManagerFacetCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::AddStake(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Fund(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::InitGenesisEpoch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Kill(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Register(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Release(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::ReleaseRewards(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ReleaseStake(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetMembership(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for GatewayManagerFacetCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AddStake(element) => ::core::fmt::Display::fmt(element, f),
                Self::Fund(element) => ::core::fmt::Display::fmt(element, f),
                Self::InitGenesisEpoch(element) => ::core::fmt::Display::fmt(element, f),
                Self::Kill(element) => ::core::fmt::Display::fmt(element, f),
                Self::Register(element) => ::core::fmt::Display::fmt(element, f),
                Self::Release(element) => ::core::fmt::Display::fmt(element, f),
                Self::ReleaseRewards(element) => ::core::fmt::Display::fmt(element, f),
                Self::ReleaseStake(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetMembership(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<AddStakeCall> for GatewayManagerFacetCalls {
        fn from(value: AddStakeCall) -> Self {
            Self::AddStake(value)
        }
    }
    impl ::core::convert::From<FundCall> for GatewayManagerFacetCalls {
        fn from(value: FundCall) -> Self {
            Self::Fund(value)
        }
    }
    impl ::core::convert::From<InitGenesisEpochCall> for GatewayManagerFacetCalls {
        fn from(value: InitGenesisEpochCall) -> Self {
            Self::InitGenesisEpoch(value)
        }
    }
    impl ::core::convert::From<KillCall> for GatewayManagerFacetCalls {
        fn from(value: KillCall) -> Self {
            Self::Kill(value)
        }
    }
    impl ::core::convert::From<RegisterCall> for GatewayManagerFacetCalls {
        fn from(value: RegisterCall) -> Self {
            Self::Register(value)
        }
    }
    impl ::core::convert::From<ReleaseCall> for GatewayManagerFacetCalls {
        fn from(value: ReleaseCall) -> Self {
            Self::Release(value)
        }
    }
    impl ::core::convert::From<ReleaseRewardsCall> for GatewayManagerFacetCalls {
        fn from(value: ReleaseRewardsCall) -> Self {
            Self::ReleaseRewards(value)
        }
    }
    impl ::core::convert::From<ReleaseStakeCall> for GatewayManagerFacetCalls {
        fn from(value: ReleaseStakeCall) -> Self {
            Self::ReleaseStake(value)
        }
    }
    impl ::core::convert::From<SetMembershipCall> for GatewayManagerFacetCalls {
        fn from(value: SetMembershipCall) -> Self {
            Self::SetMembership(value)
        }
    }
    ///`FvmAddress(uint8,bytes)`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct FvmAddress {
        pub addr_type: u8,
        pub payload: ::ethers::core::types::Bytes,
    }
    ///`SubnetID(uint64,address[])`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct SubnetID {
        pub root: u64,
        pub route: ::std::vec::Vec<::ethers::core::types::Address>,
    }
}
