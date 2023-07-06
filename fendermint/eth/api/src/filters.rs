// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use ethers_core::types::{self as et, ValueOrArray};
use fendermint_vm_actor_interface::eam::EthAddress;
use fvm_shared::address::Address;
use tendermint_rpc::{
    event::Event,
    query::{EventType, Query},
};

/// Check whether to keep a log according to the topic filter.
///
/// A note on specifying topic filters: Topics are order-dependent.
/// A transaction with a log with topics [A, B] will be matched by the following topic filters:
/// * [] "anything"
/// * [A] "A in first position (and anything after)"
/// * [null, B] "anything in first position AND B in second position (and anything after)"
/// * [A, B] "A in first position AND B in second position (and anything after)"
/// * [[A, B], [A, B]] "(A OR B) in first position AND (A OR B) in second position (and anything after)"
pub fn matches_topics(filter: &et::Filter, log: &et::Log) -> bool {
    for i in 0..4 {
        if let Some(topics) = &filter.topics[i] {
            let topic = log.topics.get(i);
            let matches = match topics {
                et::ValueOrArray::Value(Some(t)) => topic == Some(t),
                et::ValueOrArray::Array(ts) => ts.iter().flatten().any(|t| topic == Some(t)),
                _ => true,
            };
            if !matches {
                return false;
            }
        }
    }
    true
}

pub type FilterId = et::U256;

pub enum FilterKind {
    Logs(Box<et::Filter>),
    NewBlocks,
    PendingTransactions,
}

impl FilterKind {
    /// Convert an Ethereum filter to potentially multiple Tendermint queries.
    ///
    /// One limitation with Tendermint is that it only handles AND condition
    /// in filtering, so if the filter contains arrays, we have to make a
    /// cartesian product of all conditions in it and subscribe individually.
    ///
    /// https://docs.tendermint.com/v0.34/rpc/#/Websocket/subscribe
    pub fn to_queries(&self) -> anyhow::Result<Vec<Query>> {
        match self {
            FilterKind::NewBlocks => Ok(vec![Query::from(EventType::NewBlock)]),
            FilterKind::PendingTransactions => Ok(vec![Query::from(EventType::Tx)]),
            FilterKind::Logs(filter) => {
                todo!()

                // let addr = match filter.address {
                //     None => None,
                //     Some(ValueOrArray::Value(addr)) => Some(addr),
                //     Some(ValueOrArray::Array(addrs)) => {
                //         match addrs.len() {
                //             0 => None,
                //             1 => Some(addrs[0])
                //             _ => return anyhow!("Only use 1 address in a subscription.")
                //         }
                //     }
                // }

                // if let Some(addr) = filter.address {
                //     let addrs = match addr {
                //         ValueOrArray::Value(addr) => addr,
                //         ValueOrArray::Array(addrs) if addres.l
                //     }
                //     let id = Address::from(EthAddress::from(addr.0))
                //         .id()
                //         .context("Only use f0 type addresses in filters.")?;

                //     query.and_eq("emitter", id.to_string())
                // }
            }
        }
    }
}

/// Accumulate changes between polls.
#[derive(Default)]
pub struct FilterState {}

impl FilterState {
    /// Accumulate the events.
    pub fn update(&mut self, _event: Event) {
        todo!()
    }

    /// The subscription returned an error and will no longer be polled for data.
    /// Propagate the error to the reader next time it comes to check on the filter.
    pub fn finish(&mut self, _error: Option<anyhow::Error>) {
        todo!()
    }

    /// Indicate whether the reader has been too slow at polling the filter
    /// and that it should be removed.
    pub fn is_timed_out(&self) -> bool {
        todo!()
    }

    /// Indicate that the reader is no longer interested in receiving updates.
    pub fn unsubscribe(&self) -> bool {
        todo!()
    }

    /// Indicate that the reader has unsubscribed from the filter.
    pub fn is_unsubscribed(&self) -> bool {
        todo!()
    }
}
