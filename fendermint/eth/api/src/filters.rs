// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::time::{Duration, Instant};

use anyhow::Context;
use ethers_core::types as et;
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
                let mut query = Query::from(EventType::Tx);

                if let Some(block_hash) = filter.get_block_hash() {
                    query = query.and_eq("tx.hash", hex::encode(block_hash.0));
                }
                if let Some(from_block) = filter.get_from_block() {
                    query = query.and_gte("tx.height", from_block.as_u64());
                }
                if let Some(to_block) = filter.get_to_block() {
                    query = query.and_lte("tx.height", to_block.as_u64());
                }

                let mut queries = vec![query];

                let addrs = match &filter.address {
                    None => vec![],
                    Some(et::ValueOrArray::Value(addr)) => vec![*addr],
                    Some(et::ValueOrArray::Array(addrs)) => addrs.clone(),
                };

                let addrs = addrs
                    .into_iter()
                    .map(|addr| {
                        Address::from(EthAddress(addr.0))
                            .id()
                            .context("only f0 type addresses are supported")
                    })
                    .collect::<Result<Vec<u64>, _>>()?;

                if !addrs.is_empty() {
                    queries = addrs
                        .iter()
                        .flat_map(|addr| {
                            queries
                                .iter()
                                .map(|q| q.clone().and_eq("message.emitter", addr.to_string()))
                        })
                        .collect();
                };

                for i in 0..4 {
                    if let Some(Some(topics)) = filter.topics.get(i) {
                        let topics = match topics {
                            et::ValueOrArray::Value(Some(t)) => vec![t],
                            et::ValueOrArray::Array(ts) => ts.iter().flatten().collect(),
                            _ => vec![],
                        };
                        if !topics.is_empty() {
                            let key = format!("message.t{}", i + 1);
                            queries = topics
                                .into_iter()
                                .flat_map(|t| {
                                    queries
                                        .iter()
                                        .map(|q| q.clone().and_eq(&key, hex::encode(t.0)))
                                })
                                .collect();
                        }
                    }
                }

                Ok(queries)
            }
        }
    }
}

/// Accumulate changes between polls.
pub struct FilterState {
    timeout: Duration,
    last_poll: Instant,
    is_unsubscribed: bool,
    finished: Option<Option<anyhow::Error>>,
}

impl FilterState {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_poll: Instant::now(),
            is_unsubscribed: false,
            finished: None,
        }
    }

    /// Accumulate the events.
    pub fn update(&mut self, _event: Event) {}

    /// The subscription returned an error and will no longer be polled for data.
    /// Propagate the error to the reader next time it comes to check on the filter.
    pub fn finish(&mut self, error: Option<anyhow::Error>) {
        // Keep any already existing error.
        let error = self.finished.take().flatten().or(error);

        self.finished = Some(error);
    }

    /// Indicate whether the reader has been too slow at polling the filter
    /// and that it should be removed.
    pub fn is_timed_out(&self) -> bool {
        Instant::now().duration_since(self.last_poll) > self.timeout
    }

    /// Indicate that the reader is no longer interested in receiving updates.
    pub fn unsubscribe(&mut self) {
        self.is_unsubscribed = true;
    }

    /// Indicate that the reader has unsubscribed from the filter.
    pub fn is_unsubscribed(&self) -> bool {
        self.is_unsubscribed
    }
}

#[cfg(test)]
mod tests {
    use ethers_core::types as et;

    use super::FilterKind;

    #[test]
    fn filter_to_query() {
        fn hash(s: &str) -> et::H256 {
            et::H256::from(ethers_core::utils::keccak256(s))
        }

        fn hash_hex(s: &str) -> String {
            hex::encode(hash(s))
        }

        let filter = et::Filter::new()
            .select(1234..)
            .address(
                "0xff00000000000000000000000000000000000064"
                    .parse::<et::Address>()
                    .unwrap(),
            )
            .events(vec!["Foo", "Bar"])
            .topic1(hash("Alice"))
            .topic2(
                vec!["Bob", "Charlie"]
                    .into_iter()
                    .map(hash)
                    .collect::<Vec<_>>(),
            );

        eprintln!("filter = {filter:?}");

        assert_eq!(
            filter.topics[0],
            Some(et::ValueOrArray::Array(vec![
                Some(hash("Foo")),
                Some(hash("Bar"))
            ]))
        );

        let queries = FilterKind::Logs(Box::new(filter))
            .to_queries()
            .expect("failed to convert");

        assert_eq!(queries.len(), 4);

        for (i, (t1, t3)) in [
            ("Foo", "Bob"),
            ("Bar", "Bob"),
            ("Foo", "Charlie"),
            ("Bar", "Charlie"),
        ]
        .iter()
        .enumerate()
        {
            let q = queries[i].to_string();
            let e = format!("tm.event = 'Tx' AND tx.height >= 1234 AND message.emitter = '100' AND message.t1 = '{}' AND message.t2 = '{}' AND message.t3 = '{}'", hash_hex(t1), hash_hex("Alice"), hash_hex(t3));
            assert_eq!(q, e, "combination {i}");
        }
    }
}
