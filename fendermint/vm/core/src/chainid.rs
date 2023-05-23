use std::collections::HashSet;

use cid::{multihash, multihash::MultihashDigest};
use fvm_shared::bigint::{BigInt, Integer, Sign};
use fvm_shared::chainid::ChainID;
use lazy_static::lazy_static;
use thiserror::Error;

lazy_static! {
    /// Well known Filecoin chain IDs.
    ///
    /// See all EVM chain IDs at this repo: https://github.com/ethereum-lists/chains/pull/1567
    /// For now I thought it would be enough to enumerate the Filecoin ones.
    static ref KNOWN_CHAIN_IDS: HashSet<u64> = HashSet::from_iter(vec![
      0,        // Used as a default
      314,      // Filecoin
      3141,     // Hyperspace
      31415,    // Wallaby
      3141592,  // Butterlfynet
      314159,   // Calibnet
      31415926, // Devnet
    ]);
}

/// Maximum value that MetaMask and other Ethereum JS tools can safely handle.
///
/// See https://github.com/ethereum/EIPs/issues/2294
pub const MAX_CHAIN_ID: u64 = 4503599627370476;

#[derive(Error, Debug)]
pub enum ChainIDError {
    /// The name was hashed to a numeric value of a well-known chain.
    /// The chances of this are low, but if it happens, try picking a different name, if possible.
    #[error("illegal name: {0} ({1})")]
    IllegalName(String, u64),
}

/// Hash the name of the chain and reduce it to a number within the acceptable range.
pub fn from_str_hashed(name: &str) -> Result<ChainID, ChainIDError> {
    let bz = name.as_bytes();
    let digest = multihash::Code::Blake2b256.digest(bz);

    let num_digest = BigInt::from_bytes_be(Sign::Plus, digest.digest());
    let max_chain_id = BigInt::from(MAX_CHAIN_ID);

    let chain_id = num_digest.mod_floor(&max_chain_id);
    let chain_id: u64 = chain_id
        .try_into()
        .expect("modulo should be safe to convert to u64");

    if KNOWN_CHAIN_IDS.contains(&chain_id) {
        Err(ChainIDError::IllegalName(name.to_owned(), chain_id))
    } else {
        Ok(ChainID::from(chain_id))
    }
}

#[cfg(test)]
mod tests {

    use quickcheck_macros::quickcheck;

    use super::{from_str_hashed, MAX_CHAIN_ID};

    #[quickcheck]
    fn prop_chain_id_stable(name: String) -> bool {
        if let Ok(id1) = from_str_hashed(&name) {
            let id2 = from_str_hashed(&name).unwrap();
            return id1 == id2;
        }
        true
    }

    #[quickcheck]
    fn prop_chain_id_safe(name: String) -> bool {
        if let Ok(id) = from_str_hashed(&name) {
            let chain_id: u64 = id.into();
            return chain_id <= MAX_CHAIN_ID;
        }
        true
    }

    #[test]
    fn chain_id_ok() -> Result<(), String> {
        for name in vec!["test", "/root/foo/bar"] {
            if let Err(e) = from_str_hashed(name) {
                return Err(format!("failed: {name} - {e}"));
            }
        }
        Ok(())
    }
}
