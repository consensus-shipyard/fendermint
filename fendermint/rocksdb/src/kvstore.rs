// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use anyhow::anyhow;
use fendermint_storage::Decode;
use fendermint_storage::Encode;
use fendermint_storage::KVResult;
use fendermint_storage::KVTransaction;
use fendermint_storage::KVTransactionPrepared;
use fendermint_storage::KVWritable;
use fendermint_storage::KVWrite;
use fendermint_storage::{KVError, KVRead, KVReadable, KVStore};
use rocksdb::BoundColumnFamily;
use rocksdb::ErrorKind;
use rocksdb::OptimisticTransactionDB;
use rocksdb::Transaction;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::thread;

use crate::RocksDb;

/// Marker for read-only mode.
pub struct Read;
/// Marker for read-write mode.
pub struct Write;

pub struct RocksDbTx<'a, M> {
    db: &'a OptimisticTransactionDB,
    tx: ManuallyDrop<Transaction<'a, OptimisticTransactionDB>>,
    /// Cache column families to avoid further cloning on each access.
    cfs: RefCell<BTreeMap<String, Arc<BoundColumnFamily<'a>>>>,
    /// Indicate read-only or read-write mode.
    _mode: PhantomData<M>,
    /// Flag to support sanity checking in `Drop`.
    read_only: bool,
}

impl<'a, M> RocksDbTx<'a, M> {
    /// Look up a column family and pass it to a closure.
    /// Return an error if it doesn't exist.
    fn with_cf_handle<F, T>(&self, name: &str, f: F) -> KVResult<T>
    where
        F: FnOnce(&Arc<BoundColumnFamily<'a>>) -> KVResult<T>,
    {
        let mut cfs = self.cfs.borrow_mut();
        let cf = match cfs.get(name) {
            Some(cf) => cf,
            None => match self.db.cf_handle(name) {
                None => {
                    return Err(KVError::Unexpected(
                        anyhow!("column family {name} doesn't exist").into(),
                    ))
                }
                Some(cf) => {
                    cfs.insert(name.to_owned(), cf);
                    cfs.get(name).unwrap()
                }
            },
        };
        f(cf)
    }
}

impl<S> KVReadable<S> for RocksDb
where
    S: KVStore<Repr = Vec<u8>>,
    S::Namespace: AsRef<str>,
{
    type Tx<'a> = RocksDbTx<'a, Read>
    where
        Self: 'a;

    fn read(&self) -> Self::Tx<'_> {
        let tx = self.db.transaction();
        RocksDbTx {
            db: self.db.as_ref(),
            tx: ManuallyDrop::new(tx),
            cfs: Default::default(),
            read_only: true,
            _mode: PhantomData,
        }
    }
}

impl<S> KVWritable<S> for RocksDb
where
    S: KVStore<Repr = Vec<u8>>,
    S::Namespace: AsRef<str>,
{
    type Tx<'a> = RocksDbTx<'a, Write>
    where
        Self: 'a;

    fn write(&self) -> Self::Tx<'_> {
        let tx = self.db.transaction();
        RocksDbTx {
            db: self.db.as_ref(),
            tx: ManuallyDrop::new(tx),
            cfs: Default::default(),
            read_only: false,
            _mode: PhantomData,
        }
    }
}

impl<'a, S, M> KVRead<S> for RocksDbTx<'a, M>
where
    S: KVStore<Repr = Vec<u8>>,
    S::Namespace: AsRef<str>,
{
    fn get<K, V>(&self, ns: &S::Namespace, k: &K) -> KVResult<Option<V>>
    where
        S: Encode<K> + Decode<V>,
    {
        self.with_cf_handle(ns.as_ref(), |cf| {
            let key = S::to_repr(k)?;

            let res = self.tx.get_cf(cf, key.as_ref()).map_err(unexpected)?;

            match res {
                Some(bz) => Ok(Some(S::from_repr(&bz)?)),
                None => Ok(None),
            }
        })
    }
}

impl<'a, S> KVWrite<S> for RocksDbTx<'a, Write>
where
    S: KVStore<Repr = Vec<u8>>,
    S::Namespace: AsRef<str>,
{
    fn put<K, V>(&mut self, ns: &S::Namespace, k: &K, v: &V) -> KVResult<()>
    where
        S: Encode<K> + Encode<V>,
    {
        self.with_cf_handle(ns.as_ref(), |cf| {
            let k = S::to_repr(k)?;
            let v = S::to_repr(v)?;

            self.tx
                .put_cf(cf, k.as_ref(), v.as_ref())
                .map_err(unexpected)?;

            Ok(())
        })
    }

    fn delete<K>(&mut self, ns: &S::Namespace, k: &K) -> KVResult<()>
    where
        S: Encode<K>,
    {
        self.with_cf_handle(ns.as_ref(), |cf| {
            let k = S::to_repr(k)?;

            self.tx.delete_cf(cf, k.as_ref()).map_err(unexpected)?;

            Ok(())
        })
    }
}

impl<'a> KVTransaction for RocksDbTx<'a, Write> {
    type Prepared = Self;

    fn prepare(self) -> KVResult<Option<Self::Prepared>> {
        match self.tx.prepare() {
            Err(e) if e.kind() == ErrorKind::Busy => Ok(None),
            Err(e) => Err(unexpected(e)),
            Ok(()) => Ok(Some(self)),
        }
    }

    fn rollback(self) -> KVResult<()> {
        self.tx.rollback().map_err(unexpected)
    }
}

impl<'a> KVTransactionPrepared for RocksDbTx<'a, Write> {
    fn commit(self) -> KVResult<()> {
        // This method cleans up the transaction without running the panicky destructor.
        let mut this = ManuallyDrop::new(self);
        let res = unsafe {
            let tx = ManuallyDrop::take(&mut this.tx);
            tx.commit().map_err(unexpected)
        };
        res
    }

    fn rollback(self) -> KVResult<()> {
        KVTransaction::rollback(self)
    }
}

impl<'a, M> Drop for RocksDbTx<'a, M> {
    fn drop(&mut self) {
        if !self.read_only && !thread::panicking() {
            panic!("Transaction prematurely dropped. Must call `.commit()` or `.rollback()`.");
        }
    }
}

fn unexpected(e: rocksdb::Error) -> KVError {
    KVError::Unexpected(Box::new(e))
}
