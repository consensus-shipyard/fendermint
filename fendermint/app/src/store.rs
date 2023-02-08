use std::{borrow::Cow, hash::Hash, marker::PhantomData};

use fendermint_storage::{Codec, Decode, Encode, KVError, KVResult, KVStore};
use fvm_ipld_encoding::{de::DeserializeOwned, serde::Serialize};

#[derive(Clone)]
pub struct AppStore<NS> {
    _ns: PhantomData<NS>,
}

impl<NS> KVStore for AppStore<NS>
where
    NS: Eq + Hash + Clone,
{
    type Repr = Vec<u8>;
    type Namespace = NS;
}

impl<NS, T> Codec<T> for AppStore<NS> where AppStore<NS>: Encode<T> + Decode<T> {}

/// CBOR serialization.
impl<NS, T> Encode<T> for AppStore<NS>
where
    NS: Eq + Hash + Clone,
    T: Serialize,
{
    fn to_repr(value: &T) -> KVResult<Cow<Self::Repr>> {
        fvm_ipld_encoding::to_vec(value)
            .map_err(|e| KVError::Codec(Box::new(e)))
            .map(Cow::Owned)
    }
}

/// CBOR deserialization.
impl<NS, T> Decode<T> for AppStore<NS>
where
    NS: Eq + Hash + Clone,
    T: DeserializeOwned,
{
    fn from_repr(repr: &Self::Repr) -> KVResult<T> {
        fvm_ipld_encoding::from_slice(repr).map_err(|e| KVError::Codec(Box::new(e)))
    }
}
