// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::Cid;
use fvm_shared::{address::Address, clock::ChainEpoch, crypto::signature::Signature};
use ipc_sdk::subnet_id::SubnetID;
use serde::{Deserialize, Serialize};

/// Messages involved in InterPlanetary Consensus.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IpcMessage {
    /// A bottom-up checkpoint coming from a child subnet, relayed by a user of the parent subnet for a reward.
    ///
    /// The reward can be given immediately upon the validation of the quorum certificate in the checkpoint,
    /// or later during execution, once data availability has been confirmed.
    BottomUp(SignedRelayedMessage<BottomUpCheckpoint>),

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

    use quickcheck::{Arbitrary, Gen};

    use super::IpcMessage;

    impl Arbitrary for IpcMessage {
        fn arbitrary(g: &mut Gen) -> Self {
            match u8::arbitrary(g) % 1 {
                _ => todo!(),
            }
        }
    }
}
