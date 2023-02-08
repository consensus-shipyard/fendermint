mod rocks;

#[cfg(feature = "blockstore")]
mod blockstore;
#[cfg(feature = "kvstore")]
mod kvstore;

pub use rocks::{Error as RocksDbError, RocksDb, RocksDbConfig};
