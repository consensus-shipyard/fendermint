// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::Cid;
use fvm_shared::{address::Address, clock::ChainEpoch, crypto::signature::Signature};
use ipc_sdk::subnet_id::SubnetID;
use serde::{Deserialize, Serialize};

/// Messages involved in InterPlanetary Consensus.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum IpcMessage {
    /// A bottom-up checkpoint coming from a child subnet, relayed by a user of the parent subnet for a reward.
    ///
    /// The reward can be given immediately upon the validation of the quorum certificate in the checkpoint,
    /// or later during execution, once data availability has been confirmed.
    BottomUp(SignedRelayedMessage<SignedBottomUpCheckpoint>),

    // TODO
    TopDown,
}

/// A message relayed by a user on the current subnet.
///
/// The relayer pays for the inclusion of the message in the ledger,
/// but not necessarily for the execution of its contents.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayedMessage<T> {
    /// The relayed message.
    pub message: T,
    /// The address (public key) of the relayer in the current subnet.
    pub relayer: Address,
    /// The nonce of the relayer in the current subnet.
    pub sequence: u64,
}

/// Relayed messages are signed by the relayer, so we can rightfully charge them message inclusion costs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedRelayedMessage<T> {
    /// The relayed message with the relayer identity.
    pub message: RelayedMessage<T>,
    /// The signature of the relayer, for cost and reward attribution.
    pub signature: Signature,
}

/// A periodic bottom-up checkpoints contains the source subnet ID (to protect against replay attacks),
/// a block height (for sequencing), any potential handover to the next validator set, and a pointer
/// to the messages that need to be resolved and executed by the parent validators.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BottomUpCheckpoint {
    /// Which subnet is the checkpoint coming from.
    pub subnet_id: SubnetID,
    /// Block height of this checkpoint.
    pub height: ChainEpoch,
    /// Which validator set is going to sign the *next* checkpoint.
    /// The parent subnet already expects the last validator set to sign this one.
    pub next_validator_set_id: u64,
    /// Pointer at all the bottom-up messages included in this checkpoint.
    pub bottom_up_messages: Cid, // TODO: Use TCid
}

/// A bottom-up checkpoint with a quroum certificate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedBottomUpCheckpoint {
    pub checkpoint: BottomUpCheckpoint,
    pub signatures: Vec<ValidatorSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSignature {
    pub validator: Address,
    pub signature: Signature,
}

#[cfg(feature = "arb")]
mod arb {

    use fendermint_testing::arb::{ArbAddress, ArbCid, ArbSubnetID};
    use fvm_shared::crypto::signature::Signature;
    use quickcheck::{Arbitrary, Gen};

    use super::{
        BottomUpCheckpoint, IpcMessage, RelayedMessage, SignedBottomUpCheckpoint,
        SignedRelayedMessage, ValidatorSignature,
    };

    impl Arbitrary for IpcMessage {
        fn arbitrary(g: &mut Gen) -> Self {
            match u8::arbitrary(g) % 2 {
                0 => IpcMessage::BottomUp(Arbitrary::arbitrary(g)),
                _ => IpcMessage::TopDown,
            }
        }
    }

    impl<T: Arbitrary> Arbitrary for SignedRelayedMessage<T> {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                message: RelayedMessage::arbitrary(g),
                signature: Signature::arbitrary(g),
            }
        }
    }

    impl<T: Arbitrary> Arbitrary for RelayedMessage<T> {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                message: T::arbitrary(g),
                relayer: ArbAddress::arbitrary(g).0,
                sequence: u64::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedBottomUpCheckpoint {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut signatures = Vec::new();
            for _ in 0..*g.choose(&[1, 3, 5]).unwrap() {
                signatures.push(ValidatorSignature::arbitrary(g));
            }
            Self {
                checkpoint: BottomUpCheckpoint::arbitrary(g),
                signatures,
            }
        }
    }

    impl Arbitrary for ValidatorSignature {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                validator: ArbAddress::arbitrary(g).0,
                signature: Signature::arbitrary(g),
            }
        }
    }

    impl Arbitrary for BottomUpCheckpoint {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                subnet_id: ArbSubnetID::arbitrary(g).0,
                height: u32::arbitrary(g).into(),
                next_validator_set_id: Arbitrary::arbitrary(g),
                bottom_up_messages: ArbCid::arbitrary(g).0,
            }
        }
    }
}
