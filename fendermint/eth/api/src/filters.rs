// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use ethers_core::types as et;
use tendermint_rpc::{event::Event, query::Query};

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

impl From<FilterKind> for Query {
    fn from(value: FilterKind) -> Self {
        todo!()
    }
}

/// Accumulate changes between polls.
pub struct FilterState {
    id: FilterId,
}

impl FilterState {
    pub fn new(id: FilterId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &FilterId {
        &self.id
    }

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

    /// Indicate that the reader has unsubscribed from the filter.
    pub fn is_unsubscribed(&self) -> bool {
        todo!()
    }
}
