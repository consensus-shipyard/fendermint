pub use gateway_router_facet::*;
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
pub mod gateway_router_facet {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("addCheckpointSignature"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "addCheckpointSignature",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("membershipProof"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32[]"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("weight"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("signature"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
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
                    ::std::borrow::ToOwned::to_owned("applyCrossMessages"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("applyCrossMessages"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("crossMsgs"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                                ::std::boxed::Box::new(
                                                                                    ::ethers::core::abi::ethabi::ParamType::Address,
                                                                                ),
                                                                            ),
                                                                        ],
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                        ],
                                                                    ),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                                ::std::boxed::Box::new(
                                                                                    ::ethers::core::abi::ethabi::ParamType::Address,
                                                                                ),
                                                                            ),
                                                                        ],
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                        ],
                                                                    ),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(4usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Bool,
                                                ],
                                            ),
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct CrossMsg[]"),
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
                    ::std::borrow::ToOwned::to_owned("commitChildCheck"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("commitChildCheck"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commit"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                        ),
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                                ::std::vec![
                                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                                                        ::std::boxed::Box::new(
                                                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                                                        ),
                                                                                    ),
                                                                                ],
                                                                            ),
                                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                                ::std::vec![
                                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                                ],
                                                                            ),
                                                                        ],
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                                ::std::vec![
                                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                                                        ::std::boxed::Box::new(
                                                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                                                        ),
                                                                                    ),
                                                                                ],
                                                                            ),
                                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                                ::std::vec![
                                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                                ],
                                                                            ),
                                                                        ],
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(4usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Bool,
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                                        ::std::boxed::Box::new(
                                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                                        ),
                                                                    ),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                ),
                                                            ),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct BottomUpCheckpoint",
                                        ),
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
                    ::std::borrow::ToOwned::to_owned("commitParentFinality"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "commitParentFinality",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("finality"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct ParentFinality"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("n"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("validators"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ],
                                            ),
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FvmAddress[]"),
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
                (
                    ::std::borrow::ToOwned::to_owned("createBottomUpCheckpoint"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "createBottomUpCheckpoint",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("checkpoint"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                        ),
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct BottomUpCheckpointNew",
                                        ),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "membershipRootHash",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("membershipWeight"),
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
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("QuorumReached"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("QuorumReached"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("checkpoint"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("quorumWeight"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("QuorumWeightUpdated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "QuorumWeightUpdated",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("checkpoint"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("newWeight"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
            ]),
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
                    ::std::borrow::ToOwned::to_owned("CheckpointAlreadyExists"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "CheckpointAlreadyExists",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("CheckpointInfoAlreadyExists"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "CheckpointInfoAlreadyExists",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("CheckpointMembershipNotCreated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "CheckpointMembershipNotCreated",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("CheckpointNotCreated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "CheckpointNotCreated",
                            ),
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
                    ::std::borrow::ToOwned::to_owned("InconsistentPrevCheckpoint"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InconsistentPrevCheckpoint",
                            ),
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
                    ::std::borrow::ToOwned::to_owned("InvalidCheckpointEpoch"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidCheckpointEpoch",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidCheckpointSource"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidCheckpointSource",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidCrossMsgDstSubnet"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidCrossMsgDstSubnet",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidCrossMsgNonce"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidCrossMsgNonce",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidSignature"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("InvalidSignature"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotAuthorized"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotAuthorized"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
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
                    ::std::borrow::ToOwned::to_owned("NotEnoughBalance"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotEnoughBalance"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotEnoughSubnetCircSupply"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "NotEnoughSubnetCircSupply",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotInitialized"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotInitialized"),
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
                    ::std::borrow::ToOwned::to_owned("OldConfigurationNumber"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "OldConfigurationNumber",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ParentFinalityAlreadyCommitted"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ParentFinalityAlreadyCommitted",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SignatureReplay"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("SignatureReplay"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SubnetNotActive"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("SubnetNotActive"),
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
                (
                    ::std::borrow::ToOwned::to_owned("ZeroMembershipWeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ZeroMembershipWeight",
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
    pub static GATEWAYROUTERFACET_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    pub struct GatewayRouterFacet<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for GatewayRouterFacet<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for GatewayRouterFacet<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for GatewayRouterFacet<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for GatewayRouterFacet<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(GatewayRouterFacet))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> GatewayRouterFacet<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    GATEWAYROUTERFACET_ABI.clone(),
                    client,
                ),
            )
        }
        ///Calls the contract's `addCheckpointSignature` (0x2a04f220) function
        pub fn add_checkpoint_signature(
            &self,
            height: u64,
            membership_proof: ::std::vec::Vec<[u8; 32]>,
            weight: ::ethers::core::types::U256,
            signature: ::ethers::core::types::Bytes,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [42, 4, 242, 32],
                    (height, membership_proof, weight, signature),
                )
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `applyCrossMessages` (0x3dde36ec) function
        pub fn apply_cross_messages(
            &self,
            cross_msgs: ::std::vec::Vec<CrossMsg>,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([61, 222, 54, 236], cross_msgs)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `commitChildCheck` (0xd4e149a8) function
        pub fn commit_child_check(
            &self,
            commit: BottomUpCheckpoint,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([212, 225, 73, 168], (commit,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `commitParentFinality` (0x9fa68440) function
        pub fn commit_parent_finality(
            &self,
            finality: ParentFinality,
            n: u64,
            validators: ::std::vec::Vec<FvmAddress>,
            weights: ::std::vec::Vec<::ethers::core::types::U256>,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([159, 166, 132, 64], (finality, n, validators, weights))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `createBottomUpCheckpoint` (0xa656ca5b) function
        pub fn create_bottom_up_checkpoint(
            &self,
            checkpoint: BottomUpCheckpointNew,
            membership_root_hash: [u8; 32],
            membership_weight: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [166, 86, 202, 91],
                    (checkpoint, membership_root_hash, membership_weight),
                )
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `QuorumReached` event
        pub fn quorum_reached_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            QuorumReachedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `QuorumWeightUpdated` event
        pub fn quorum_weight_updated_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            QuorumWeightUpdatedFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            GatewayRouterFacetEvents,
        > {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for GatewayRouterFacet<M> {
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
    ///Custom Error type `CheckpointAlreadyExists` with signature `CheckpointAlreadyExists()` and selector `0xb8a1eae1`
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
    #[etherror(name = "CheckpointAlreadyExists", abi = "CheckpointAlreadyExists()")]
    pub struct CheckpointAlreadyExists;
    ///Custom Error type `CheckpointInfoAlreadyExists` with signature `CheckpointInfoAlreadyExists()` and selector `0xa04ff7c7`
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
        name = "CheckpointInfoAlreadyExists",
        abi = "CheckpointInfoAlreadyExists()"
    )]
    pub struct CheckpointInfoAlreadyExists;
    ///Custom Error type `CheckpointMembershipNotCreated` with signature `CheckpointMembershipNotCreated()` and selector `0x009f833a`
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
        name = "CheckpointMembershipNotCreated",
        abi = "CheckpointMembershipNotCreated()"
    )]
    pub struct CheckpointMembershipNotCreated;
    ///Custom Error type `CheckpointNotCreated` with signature `CheckpointNotCreated()` and selector `0x58cdd2e7`
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
    #[etherror(name = "CheckpointNotCreated", abi = "CheckpointNotCreated()")]
    pub struct CheckpointNotCreated;
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
    ///Custom Error type `InconsistentPrevCheckpoint` with signature `InconsistentPrevCheckpoint()` and selector `0x24465cba`
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
        name = "InconsistentPrevCheckpoint",
        abi = "InconsistentPrevCheckpoint()"
    )]
    pub struct InconsistentPrevCheckpoint;
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
    ///Custom Error type `InvalidCheckpointEpoch` with signature `InvalidCheckpointEpoch()` and selector `0xfae4eadb`
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
    #[etherror(name = "InvalidCheckpointEpoch", abi = "InvalidCheckpointEpoch()")]
    pub struct InvalidCheckpointEpoch;
    ///Custom Error type `InvalidCheckpointSource` with signature `InvalidCheckpointSource()` and selector `0xfe72264e`
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
    #[etherror(name = "InvalidCheckpointSource", abi = "InvalidCheckpointSource()")]
    pub struct InvalidCheckpointSource;
    ///Custom Error type `InvalidCrossMsgDstSubnet` with signature `InvalidCrossMsgDstSubnet()` and selector `0xc5f563eb`
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
    #[etherror(name = "InvalidCrossMsgDstSubnet", abi = "InvalidCrossMsgDstSubnet()")]
    pub struct InvalidCrossMsgDstSubnet;
    ///Custom Error type `InvalidCrossMsgNonce` with signature `InvalidCrossMsgNonce()` and selector `0xa57cadff`
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
    #[etherror(name = "InvalidCrossMsgNonce", abi = "InvalidCrossMsgNonce()")]
    pub struct InvalidCrossMsgNonce;
    ///Custom Error type `InvalidSignature` with signature `InvalidSignature()` and selector `0x8baa579f`
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
    #[etherror(name = "InvalidSignature", abi = "InvalidSignature()")]
    pub struct InvalidSignature;
    ///Custom Error type `NotAuthorized` with signature `NotAuthorized(address)` and selector `0x4a0bfec1`
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
    #[etherror(name = "NotAuthorized", abi = "NotAuthorized(address)")]
    pub struct NotAuthorized(pub ::ethers::core::types::Address);
    ///Custom Error type `NotEnoughBalance` with signature `NotEnoughBalance()` and selector `0xad3a8b9e`
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
    #[etherror(name = "NotEnoughBalance", abi = "NotEnoughBalance()")]
    pub struct NotEnoughBalance;
    ///Custom Error type `NotEnoughSubnetCircSupply` with signature `NotEnoughSubnetCircSupply()` and selector `0x74db2854`
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
    #[etherror(name = "NotEnoughSubnetCircSupply", abi = "NotEnoughSubnetCircSupply()")]
    pub struct NotEnoughSubnetCircSupply;
    ///Custom Error type `NotInitialized` with signature `NotInitialized()` and selector `0x87138d5c`
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
    #[etherror(name = "NotInitialized", abi = "NotInitialized()")]
    pub struct NotInitialized;
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
    ///Custom Error type `OldConfigurationNumber` with signature `OldConfigurationNumber()` and selector `0x6e8d7c4a`
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
    #[etherror(name = "OldConfigurationNumber", abi = "OldConfigurationNumber()")]
    pub struct OldConfigurationNumber;
    ///Custom Error type `ParentFinalityAlreadyCommitted` with signature `ParentFinalityAlreadyCommitted()` and selector `0x2a75b082`
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
        name = "ParentFinalityAlreadyCommitted",
        abi = "ParentFinalityAlreadyCommitted()"
    )]
    pub struct ParentFinalityAlreadyCommitted;
    ///Custom Error type `SignatureReplay` with signature `SignatureReplay()` and selector `0xb47fa1b2`
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
    #[etherror(name = "SignatureReplay", abi = "SignatureReplay()")]
    pub struct SignatureReplay;
    ///Custom Error type `SubnetNotActive` with signature `SubnetNotActive()` and selector `0xc18316bf`
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
    #[etherror(name = "SubnetNotActive", abi = "SubnetNotActive()")]
    pub struct SubnetNotActive;
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
    ///Custom Error type `ZeroMembershipWeight` with signature `ZeroMembershipWeight()` and selector `0x4e8ac6e5`
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
    #[etherror(name = "ZeroMembershipWeight", abi = "ZeroMembershipWeight()")]
    pub struct ZeroMembershipWeight;
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum GatewayRouterFacetErrors {
        AddressEmptyCode(AddressEmptyCode),
        AddressInsufficientBalance(AddressInsufficientBalance),
        CheckpointAlreadyExists(CheckpointAlreadyExists),
        CheckpointInfoAlreadyExists(CheckpointInfoAlreadyExists),
        CheckpointMembershipNotCreated(CheckpointMembershipNotCreated),
        CheckpointNotCreated(CheckpointNotCreated),
        FailedInnerCall(FailedInnerCall),
        InconsistentPrevCheckpoint(InconsistentPrevCheckpoint),
        InvalidActorAddress(InvalidActorAddress),
        InvalidCheckpointEpoch(InvalidCheckpointEpoch),
        InvalidCheckpointSource(InvalidCheckpointSource),
        InvalidCrossMsgDstSubnet(InvalidCrossMsgDstSubnet),
        InvalidCrossMsgNonce(InvalidCrossMsgNonce),
        InvalidSignature(InvalidSignature),
        NotAuthorized(NotAuthorized),
        NotEnoughBalance(NotEnoughBalance),
        NotEnoughSubnetCircSupply(NotEnoughSubnetCircSupply),
        NotInitialized(NotInitialized),
        NotRegisteredSubnet(NotRegisteredSubnet),
        NotSystemActor(NotSystemActor),
        OldConfigurationNumber(OldConfigurationNumber),
        ParentFinalityAlreadyCommitted(ParentFinalityAlreadyCommitted),
        SignatureReplay(SignatureReplay),
        SubnetNotActive(SubnetNotActive),
        ValidatorWeightIsZero(ValidatorWeightIsZero),
        ValidatorsAndWeightsLengthMismatch(ValidatorsAndWeightsLengthMismatch),
        ZeroMembershipWeight(ZeroMembershipWeight),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for GatewayRouterFacetErrors {
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
                = <CheckpointAlreadyExists as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CheckpointAlreadyExists(decoded));
            }
            if let Ok(decoded)
                = <CheckpointInfoAlreadyExists as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CheckpointInfoAlreadyExists(decoded));
            }
            if let Ok(decoded)
                = <CheckpointMembershipNotCreated as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CheckpointMembershipNotCreated(decoded));
            }
            if let Ok(decoded)
                = <CheckpointNotCreated as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CheckpointNotCreated(decoded));
            }
            if let Ok(decoded)
                = <FailedInnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::FailedInnerCall(decoded));
            }
            if let Ok(decoded)
                = <InconsistentPrevCheckpoint as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InconsistentPrevCheckpoint(decoded));
            }
            if let Ok(decoded)
                = <InvalidActorAddress as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::InvalidActorAddress(decoded));
            }
            if let Ok(decoded)
                = <InvalidCheckpointEpoch as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InvalidCheckpointEpoch(decoded));
            }
            if let Ok(decoded)
                = <InvalidCheckpointSource as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InvalidCheckpointSource(decoded));
            }
            if let Ok(decoded)
                = <InvalidCrossMsgDstSubnet as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InvalidCrossMsgDstSubnet(decoded));
            }
            if let Ok(decoded)
                = <InvalidCrossMsgNonce as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::InvalidCrossMsgNonce(decoded));
            }
            if let Ok(decoded)
                = <InvalidSignature as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::InvalidSignature(decoded));
            }
            if let Ok(decoded)
                = <NotAuthorized as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotAuthorized(decoded));
            }
            if let Ok(decoded)
                = <NotEnoughBalance as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotEnoughBalance(decoded));
            }
            if let Ok(decoded)
                = <NotEnoughSubnetCircSupply as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::NotEnoughSubnetCircSupply(decoded));
            }
            if let Ok(decoded)
                = <NotInitialized as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotInitialized(decoded));
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
                = <OldConfigurationNumber as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::OldConfigurationNumber(decoded));
            }
            if let Ok(decoded)
                = <ParentFinalityAlreadyCommitted as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::ParentFinalityAlreadyCommitted(decoded));
            }
            if let Ok(decoded)
                = <SignatureReplay as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::SignatureReplay(decoded));
            }
            if let Ok(decoded)
                = <SubnetNotActive as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::SubnetNotActive(decoded));
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
            if let Ok(decoded)
                = <ZeroMembershipWeight as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::ZeroMembershipWeight(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for GatewayRouterFacetErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::AddressEmptyCode(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::AddressInsufficientBalance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CheckpointAlreadyExists(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CheckpointInfoAlreadyExists(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CheckpointMembershipNotCreated(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CheckpointNotCreated(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::FailedInnerCall(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InconsistentPrevCheckpoint(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidActorAddress(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidCheckpointEpoch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidCheckpointSource(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidCrossMsgDstSubnet(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidCrossMsgNonce(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidSignature(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotAuthorized(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEnoughBalance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotEnoughSubnetCircSupply(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotInitialized(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotRegisteredSubnet(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotSystemActor(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OldConfigurationNumber(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ParentFinalityAlreadyCommitted(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SignatureReplay(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SubnetNotActive(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorWeightIsZero(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorsAndWeightsLengthMismatch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ZeroMembershipWeight(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for GatewayRouterFacetErrors {
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
                    == <CheckpointAlreadyExists as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <CheckpointInfoAlreadyExists as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <CheckpointMembershipNotCreated as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <CheckpointNotCreated as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <FailedInnerCall as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InconsistentPrevCheckpoint as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidActorAddress as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidCheckpointEpoch as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidCheckpointSource as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidCrossMsgDstSubnet as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidCrossMsgNonce as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <InvalidSignature as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotAuthorized as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotEnoughBalance as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotEnoughSubnetCircSupply as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotInitialized as ::ethers::contract::EthError>::selector() => {
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
                    == <OldConfigurationNumber as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ParentFinalityAlreadyCommitted as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <SignatureReplay as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <SubnetNotActive as ::ethers::contract::EthError>::selector() => {
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
                _ if selector
                    == <ZeroMembershipWeight as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for GatewayRouterFacetErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AddressEmptyCode(element) => ::core::fmt::Display::fmt(element, f),
                Self::AddressInsufficientBalance(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CheckpointAlreadyExists(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CheckpointInfoAlreadyExists(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CheckpointMembershipNotCreated(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CheckpointNotCreated(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::FailedInnerCall(element) => ::core::fmt::Display::fmt(element, f),
                Self::InconsistentPrevCheckpoint(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidActorAddress(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidCheckpointEpoch(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidCheckpointSource(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidCrossMsgDstSubnet(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidCrossMsgNonce(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::InvalidSignature(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotAuthorized(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotEnoughBalance(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotEnoughSubnetCircSupply(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotInitialized(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotRegisteredSubnet(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::NotSystemActor(element) => ::core::fmt::Display::fmt(element, f),
                Self::OldConfigurationNumber(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ParentFinalityAlreadyCommitted(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::SignatureReplay(element) => ::core::fmt::Display::fmt(element, f),
                Self::SubnetNotActive(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorWeightIsZero(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ValidatorsAndWeightsLengthMismatch(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ZeroMembershipWeight(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for GatewayRouterFacetErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<AddressEmptyCode> for GatewayRouterFacetErrors {
        fn from(value: AddressEmptyCode) -> Self {
            Self::AddressEmptyCode(value)
        }
    }
    impl ::core::convert::From<AddressInsufficientBalance> for GatewayRouterFacetErrors {
        fn from(value: AddressInsufficientBalance) -> Self {
            Self::AddressInsufficientBalance(value)
        }
    }
    impl ::core::convert::From<CheckpointAlreadyExists> for GatewayRouterFacetErrors {
        fn from(value: CheckpointAlreadyExists) -> Self {
            Self::CheckpointAlreadyExists(value)
        }
    }
    impl ::core::convert::From<CheckpointInfoAlreadyExists>
    for GatewayRouterFacetErrors {
        fn from(value: CheckpointInfoAlreadyExists) -> Self {
            Self::CheckpointInfoAlreadyExists(value)
        }
    }
    impl ::core::convert::From<CheckpointMembershipNotCreated>
    for GatewayRouterFacetErrors {
        fn from(value: CheckpointMembershipNotCreated) -> Self {
            Self::CheckpointMembershipNotCreated(value)
        }
    }
    impl ::core::convert::From<CheckpointNotCreated> for GatewayRouterFacetErrors {
        fn from(value: CheckpointNotCreated) -> Self {
            Self::CheckpointNotCreated(value)
        }
    }
    impl ::core::convert::From<FailedInnerCall> for GatewayRouterFacetErrors {
        fn from(value: FailedInnerCall) -> Self {
            Self::FailedInnerCall(value)
        }
    }
    impl ::core::convert::From<InconsistentPrevCheckpoint> for GatewayRouterFacetErrors {
        fn from(value: InconsistentPrevCheckpoint) -> Self {
            Self::InconsistentPrevCheckpoint(value)
        }
    }
    impl ::core::convert::From<InvalidActorAddress> for GatewayRouterFacetErrors {
        fn from(value: InvalidActorAddress) -> Self {
            Self::InvalidActorAddress(value)
        }
    }
    impl ::core::convert::From<InvalidCheckpointEpoch> for GatewayRouterFacetErrors {
        fn from(value: InvalidCheckpointEpoch) -> Self {
            Self::InvalidCheckpointEpoch(value)
        }
    }
    impl ::core::convert::From<InvalidCheckpointSource> for GatewayRouterFacetErrors {
        fn from(value: InvalidCheckpointSource) -> Self {
            Self::InvalidCheckpointSource(value)
        }
    }
    impl ::core::convert::From<InvalidCrossMsgDstSubnet> for GatewayRouterFacetErrors {
        fn from(value: InvalidCrossMsgDstSubnet) -> Self {
            Self::InvalidCrossMsgDstSubnet(value)
        }
    }
    impl ::core::convert::From<InvalidCrossMsgNonce> for GatewayRouterFacetErrors {
        fn from(value: InvalidCrossMsgNonce) -> Self {
            Self::InvalidCrossMsgNonce(value)
        }
    }
    impl ::core::convert::From<InvalidSignature> for GatewayRouterFacetErrors {
        fn from(value: InvalidSignature) -> Self {
            Self::InvalidSignature(value)
        }
    }
    impl ::core::convert::From<NotAuthorized> for GatewayRouterFacetErrors {
        fn from(value: NotAuthorized) -> Self {
            Self::NotAuthorized(value)
        }
    }
    impl ::core::convert::From<NotEnoughBalance> for GatewayRouterFacetErrors {
        fn from(value: NotEnoughBalance) -> Self {
            Self::NotEnoughBalance(value)
        }
    }
    impl ::core::convert::From<NotEnoughSubnetCircSupply> for GatewayRouterFacetErrors {
        fn from(value: NotEnoughSubnetCircSupply) -> Self {
            Self::NotEnoughSubnetCircSupply(value)
        }
    }
    impl ::core::convert::From<NotInitialized> for GatewayRouterFacetErrors {
        fn from(value: NotInitialized) -> Self {
            Self::NotInitialized(value)
        }
    }
    impl ::core::convert::From<NotRegisteredSubnet> for GatewayRouterFacetErrors {
        fn from(value: NotRegisteredSubnet) -> Self {
            Self::NotRegisteredSubnet(value)
        }
    }
    impl ::core::convert::From<NotSystemActor> for GatewayRouterFacetErrors {
        fn from(value: NotSystemActor) -> Self {
            Self::NotSystemActor(value)
        }
    }
    impl ::core::convert::From<OldConfigurationNumber> for GatewayRouterFacetErrors {
        fn from(value: OldConfigurationNumber) -> Self {
            Self::OldConfigurationNumber(value)
        }
    }
    impl ::core::convert::From<ParentFinalityAlreadyCommitted>
    for GatewayRouterFacetErrors {
        fn from(value: ParentFinalityAlreadyCommitted) -> Self {
            Self::ParentFinalityAlreadyCommitted(value)
        }
    }
    impl ::core::convert::From<SignatureReplay> for GatewayRouterFacetErrors {
        fn from(value: SignatureReplay) -> Self {
            Self::SignatureReplay(value)
        }
    }
    impl ::core::convert::From<SubnetNotActive> for GatewayRouterFacetErrors {
        fn from(value: SubnetNotActive) -> Self {
            Self::SubnetNotActive(value)
        }
    }
    impl ::core::convert::From<ValidatorWeightIsZero> for GatewayRouterFacetErrors {
        fn from(value: ValidatorWeightIsZero) -> Self {
            Self::ValidatorWeightIsZero(value)
        }
    }
    impl ::core::convert::From<ValidatorsAndWeightsLengthMismatch>
    for GatewayRouterFacetErrors {
        fn from(value: ValidatorsAndWeightsLengthMismatch) -> Self {
            Self::ValidatorsAndWeightsLengthMismatch(value)
        }
    }
    impl ::core::convert::From<ZeroMembershipWeight> for GatewayRouterFacetErrors {
        fn from(value: ZeroMembershipWeight) -> Self {
            Self::ZeroMembershipWeight(value)
        }
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "QuorumReached", abi = "QuorumReached(uint64,bytes32,uint256)")]
    pub struct QuorumReachedFilter {
        pub height: u64,
        pub checkpoint: [u8; 32],
        pub quorum_weight: ::ethers::core::types::U256,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(
        name = "QuorumWeightUpdated",
        abi = "QuorumWeightUpdated(uint64,bytes32,uint256)"
    )]
    pub struct QuorumWeightUpdatedFilter {
        pub height: u64,
        pub checkpoint: [u8; 32],
        pub new_weight: ::ethers::core::types::U256,
    }
    ///Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum GatewayRouterFacetEvents {
        QuorumReachedFilter(QuorumReachedFilter),
        QuorumWeightUpdatedFilter(QuorumWeightUpdatedFilter),
    }
    impl ::ethers::contract::EthLogDecode for GatewayRouterFacetEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = QuorumReachedFilter::decode_log(log) {
                return Ok(GatewayRouterFacetEvents::QuorumReachedFilter(decoded));
            }
            if let Ok(decoded) = QuorumWeightUpdatedFilter::decode_log(log) {
                return Ok(GatewayRouterFacetEvents::QuorumWeightUpdatedFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for GatewayRouterFacetEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::QuorumReachedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::QuorumWeightUpdatedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
            }
        }
    }
    impl ::core::convert::From<QuorumReachedFilter> for GatewayRouterFacetEvents {
        fn from(value: QuorumReachedFilter) -> Self {
            Self::QuorumReachedFilter(value)
        }
    }
    impl ::core::convert::From<QuorumWeightUpdatedFilter> for GatewayRouterFacetEvents {
        fn from(value: QuorumWeightUpdatedFilter) -> Self {
            Self::QuorumWeightUpdatedFilter(value)
        }
    }
    ///Container type for all input parameters for the `addCheckpointSignature` function with signature `addCheckpointSignature(uint64,bytes32[],uint256,bytes)` and selector `0x2a04f220`
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
    #[ethcall(
        name = "addCheckpointSignature",
        abi = "addCheckpointSignature(uint64,bytes32[],uint256,bytes)"
    )]
    pub struct AddCheckpointSignatureCall {
        pub height: u64,
        pub membership_proof: ::std::vec::Vec<[u8; 32]>,
        pub weight: ::ethers::core::types::U256,
        pub signature: ::ethers::core::types::Bytes,
    }
    ///Container type for all input parameters for the `applyCrossMessages` function with signature `applyCrossMessages(((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)[])` and selector `0x3dde36ec`
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
    #[ethcall(
        name = "applyCrossMessages",
        abi = "applyCrossMessages(((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)[])"
    )]
    pub struct ApplyCrossMessagesCall {
        pub cross_msgs: ::std::vec::Vec<CrossMsg>,
    }
    ///Container type for all input parameters for the `commitChildCheck` function with signature `commitChildCheck(((uint64,address[]),uint64,uint256,((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)[],((uint64,address[]),bytes32[])[],bytes32,bytes))` and selector `0xd4e149a8`
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
    #[ethcall(
        name = "commitChildCheck",
        abi = "commitChildCheck(((uint64,address[]),uint64,uint256,((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)[],((uint64,address[]),bytes32[])[],bytes32,bytes))"
    )]
    pub struct CommitChildCheckCall {
        pub commit: BottomUpCheckpoint,
    }
    ///Container type for all input parameters for the `commitParentFinality` function with signature `commitParentFinality((uint256,bytes32),uint64,(uint8,bytes)[],uint256[])` and selector `0x9fa68440`
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
    #[ethcall(
        name = "commitParentFinality",
        abi = "commitParentFinality((uint256,bytes32),uint64,(uint8,bytes)[],uint256[])"
    )]
    pub struct CommitParentFinalityCall {
        pub finality: ParentFinality,
        pub n: u64,
        pub validators: ::std::vec::Vec<FvmAddress>,
        pub weights: ::std::vec::Vec<::ethers::core::types::U256>,
    }
    ///Container type for all input parameters for the `createBottomUpCheckpoint` function with signature `createBottomUpCheckpoint(((uint64,address[]),uint64,bytes32,uint64,bytes32),bytes32,uint256)` and selector `0xa656ca5b`
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
    #[ethcall(
        name = "createBottomUpCheckpoint",
        abi = "createBottomUpCheckpoint(((uint64,address[]),uint64,bytes32,uint64,bytes32),bytes32,uint256)"
    )]
    pub struct CreateBottomUpCheckpointCall {
        pub checkpoint: BottomUpCheckpointNew,
        pub membership_root_hash: [u8; 32],
        pub membership_weight: ::ethers::core::types::U256,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum GatewayRouterFacetCalls {
        AddCheckpointSignature(AddCheckpointSignatureCall),
        ApplyCrossMessages(ApplyCrossMessagesCall),
        CommitChildCheck(CommitChildCheckCall),
        CommitParentFinality(CommitParentFinalityCall),
        CreateBottomUpCheckpoint(CreateBottomUpCheckpointCall),
    }
    impl ::ethers::core::abi::AbiDecode for GatewayRouterFacetCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded)
                = <AddCheckpointSignatureCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::AddCheckpointSignature(decoded));
            }
            if let Ok(decoded)
                = <ApplyCrossMessagesCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::ApplyCrossMessages(decoded));
            }
            if let Ok(decoded)
                = <CommitChildCheckCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CommitChildCheck(decoded));
            }
            if let Ok(decoded)
                = <CommitParentFinalityCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CommitParentFinality(decoded));
            }
            if let Ok(decoded)
                = <CreateBottomUpCheckpointCall as ::ethers::core::abi::AbiDecode>::decode(
                    data,
                ) {
                return Ok(Self::CreateBottomUpCheckpoint(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for GatewayRouterFacetCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::AddCheckpointSignature(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ApplyCrossMessages(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CommitChildCheck(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CommitParentFinality(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::CreateBottomUpCheckpoint(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for GatewayRouterFacetCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AddCheckpointSignature(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ApplyCrossMessages(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CommitChildCheck(element) => ::core::fmt::Display::fmt(element, f),
                Self::CommitParentFinality(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::CreateBottomUpCheckpoint(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
            }
        }
    }
    impl ::core::convert::From<AddCheckpointSignatureCall> for GatewayRouterFacetCalls {
        fn from(value: AddCheckpointSignatureCall) -> Self {
            Self::AddCheckpointSignature(value)
        }
    }
    impl ::core::convert::From<ApplyCrossMessagesCall> for GatewayRouterFacetCalls {
        fn from(value: ApplyCrossMessagesCall) -> Self {
            Self::ApplyCrossMessages(value)
        }
    }
    impl ::core::convert::From<CommitChildCheckCall> for GatewayRouterFacetCalls {
        fn from(value: CommitChildCheckCall) -> Self {
            Self::CommitChildCheck(value)
        }
    }
    impl ::core::convert::From<CommitParentFinalityCall> for GatewayRouterFacetCalls {
        fn from(value: CommitParentFinalityCall) -> Self {
            Self::CommitParentFinality(value)
        }
    }
    impl ::core::convert::From<CreateBottomUpCheckpointCall>
    for GatewayRouterFacetCalls {
        fn from(value: CreateBottomUpCheckpointCall) -> Self {
            Self::CreateBottomUpCheckpoint(value)
        }
    }
    ///`BottomUpCheckpoint((uint64,address[]),uint64,uint256,((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)[],((uint64,address[]),bytes32[])[],bytes32,bytes)`
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
    pub struct BottomUpCheckpoint {
        pub source: SubnetID,
        pub epoch: u64,
        pub fee: ::ethers::core::types::U256,
        pub cross_msgs: ::std::vec::Vec<CrossMsg>,
        pub children: ::std::vec::Vec<ChildCheck>,
        pub prev_hash: [u8; 32],
        pub proof: ::ethers::core::types::Bytes,
    }
    ///`BottomUpCheckpointNew((uint64,address[]),uint64,bytes32,uint64,bytes32)`
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
    pub struct BottomUpCheckpointNew {
        pub subnet_id: SubnetID,
        pub block_height: u64,
        pub block_hash: [u8; 32],
        pub next_configuration_number: u64,
        pub cross_messages_hash: [u8; 32],
    }
    ///`ChildCheck((uint64,address[]),bytes32[])`
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
    pub struct ChildCheck {
        pub source: SubnetID,
        pub checks: ::std::vec::Vec<[u8; 32]>,
    }
    ///`CrossMsg((((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes),bool)`
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
    pub struct CrossMsg {
        pub message: StorableMsg,
        pub wrapped: bool,
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
    ///`Ipcaddress((uint64,address[]),(uint8,bytes))`
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
    pub struct Ipcaddress {
        pub subnet_id: SubnetID,
        pub raw_address: FvmAddress,
    }
    ///`ParentFinality(uint256,bytes32)`
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
    pub struct ParentFinality {
        pub height: ::ethers::core::types::U256,
        pub block_hash: [u8; 32],
    }
    ///`StorableMsg(((uint64,address[]),(uint8,bytes)),((uint64,address[]),(uint8,bytes)),uint256,uint64,bytes4,bytes)`
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
    pub struct StorableMsg {
        pub from: Ipcaddress,
        pub to: Ipcaddress,
        pub value: ::ethers::core::types::U256,
        pub nonce: u64,
        pub method: [u8; 4],
        pub params: ::ethers::core::types::Bytes,
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
