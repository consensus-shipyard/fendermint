// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
// For benchmarks.
use std::error::Error;
use std::hash::Hash;
use std::marker::PhantomData;

/// In-memory KV store backend.
#[cfg(feature = "inmem")]
pub mod im;

/// Possible errors during key-value operations.
#[derive(Debug)]
pub enum KVError {
    /// KV transaction was aborted due to some business rule violation.
    Abort(Box<dyn Error + Send + Sync>),
    /// An error occurred during serializing or deserializing the data.
    Codec(Box<dyn Error + Send + Sync>),
    /// Some unexpected error occurred in the underlying implementation,
    /// e.g. some IO error with a database.
    Unexpected(Box<dyn Error + Send + Sync>),
}

pub type KVResult<T> = Result<T, KVError>;

/// Helper trait to reduce the number of generic parameters that infect anything
/// that has to use a KV store. It's a type family of all customizable types
/// that can vary by KV store implementation.
pub trait KVStore {
    /// Type specifying in which collection to store some homogenous data set.
    type Namespace: Clone + Hash + Eq;

    /// The type used for storing data at rest, e.g. in binary format or JSON.
    type Repr: Clone;
}

/// Encode data as binary with a serialization scheme.
pub trait Encode<T>
where
    Self: KVStore,
{
    fn to_repr(value: &T) -> KVResult<Self::Repr>;
}

/// Decode data from binary with a serialization scheme.
pub trait Decode<T>
where
    Self: KVStore,
{
    fn from_repr(repr: &Self::Repr) -> KVResult<T>;
}

/// Encode and decode data.
///
/// Ideally this would be just a trait alias, but that's an unstable feature.
pub trait Codec<T>: Encode<T> + Decode<T> {}

/// Operations available on a read transaction.
pub trait KVRead<S: KVStore> {
    fn get<K, V>(&self, ns: &S::Namespace, k: &K) -> KVResult<Option<V>>
    where
        S: Encode<K> + Decode<V>;
}

/// Operations available on a write transaction.
pub trait KVWrite<S: KVStore>: KVRead<S> {
    fn put<K, V>(&mut self, ns: &S::Namespace, k: &K, v: &V) -> KVResult<()>
    where
        S: Encode<K> + Encode<V>;

    fn delete<K>(&mut self, ns: &S::Namespace, k: &K) -> KVResult<()>
    where
        S: Encode<K>;
}

/// Transaction running on a KV store, ending with a commit or a rollback.
/// This mimics the `Aux` interface in the STM module.
pub trait KVTransaction {
    type Prepared: KVTransactionPrepared;
    /// Prepare to commit the transaction. This gives us a chance to do
    /// Optimistic Concurrency Control, to only take out locks during commit.
    fn prepare(self) -> Option<Self::Prepared>;

    /// Abandon the changes of the transaction.
    fn rollback(self);

    /// Convenience method to prepare and commit.
    ///
    /// Returns a flag indicating whether the commit successful.
    fn prepare_and_commit(self) -> bool
    where
        Self: Sized,
    {
        self.prepare().map(|tx| tx.commit()).is_some()
    }
}

/// Transaction in a state when it's ready to be committed.
pub trait KVTransactionPrepared {
    fn commit(self);
    fn rollback(self);
}

/// Interface for stores that support read-only transactions.
///
/// Any resources held by the read transaction should be released when it's dropped.
pub trait KVReadable<S: KVStore> {
    type Tx<'a>: KVRead<S>
    where
        Self: 'a;

    /// Start a read-only transaction.
    fn read(&self) -> Self::Tx<'_>;
}

/// Interface for stores that support read-write transactions.
pub trait KVWritable<S: KVStore> {
    type Tx<'a>: KVWrite<S> + KVTransaction
    where
        Self: 'a;

    /// Start a read-write tranasction.
    fn write(&self) -> Self::Tx<'_>;
}

/// A collection of homogenous objects under the same namespace.
#[derive(Clone)]
pub struct KVCollection<S: KVStore, K, V> {
    ns: S::Namespace,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<S: KVStore, K, V> KVCollection<S, K, V>
where
    S: Encode<K> + Encode<V> + Decode<V>,
{
    pub fn new(ns: S::Namespace) -> Self {
        Self {
            ns,
            phantom_k: PhantomData,
            phantom_v: PhantomData,
        }
    }

    pub fn get(&self, kv: &impl KVRead<S>, k: &K) -> KVResult<Option<V>> {
        kv.get(&self.ns, k)
    }

    pub fn put(&self, kv: &mut impl KVWrite<S>, k: &K, v: &V) -> KVResult<()> {
        kv.put(&self.ns, k, v)
    }

    pub fn delete(&self, kv: &mut impl KVWrite<S>, k: &K) -> KVResult<()> {
        kv.delete(&self.ns, k)
    }
}

#[cfg(test)]
mod tests;
