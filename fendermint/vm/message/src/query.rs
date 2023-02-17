use cid::Cid;
use fvm_shared::{address::Address, econ::TokenAmount, ActorID};
use serde::{Deserialize, Serialize};

/// Queries over the IPLD blockstore or the state tree.
///
/// Maybe we can have some common queries over the known state of built-in actors,
/// and actors supporting IPC, or FEVM.
#[derive(Serialize, Deserialize)]
pub enum FvmQuery {
    /// Query something from the IPLD store.
    Ipld(Cid),
    /// Query the state of an actor.
    ActorState(Address),
}

pub enum FvmQueryRet {
    /// Bytes from the IPLD store retult, if found.
    Ipld(Option<Vec<u8>>),
    /// The full state of an actor, if found.
    ActorState(Option<Box<(ActorID, ActorState)>>),
}

/// State of all actor implementations.
///
/// This is a copy of `fvm::state_tree::ActorState` so that this crate
/// doesn't need a dependency on `fvm` itself, only `fvm_shared`.
///
/// I changed `Serialize_tuple` into `Serialize` - could be better as a
/// message exchange format if the field names are in tact.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ActorState {
    /// Link to code for the actor.
    pub code: Cid,
    /// Link to the state of the actor.
    pub state: Cid,
    /// Sequence of the actor.
    pub sequence: u64,
    /// Tokens available to the actor.
    pub balance: TokenAmount,
    /// The actor's "delegated" address, if assigned.
    ///
    /// This field is set on actor creation and never modified.
    pub delegated_address: Option<Address>,
}
