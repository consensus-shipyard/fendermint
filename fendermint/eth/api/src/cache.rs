// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::Context;
use fendermint_rpc::client::FendermintClient;
use fendermint_rpc::query::QueryClient;
use fvm_shared::{
    address::{Address, Payload},
    ActorID,
};
use tendermint_rpc::Client;

/// Facilitate Ethereum address <-> Actor ID lookups.
#[derive(Clone)]
pub struct AddressCache<C> {
    client: FendermintClient<C>,
    addr_to_id: Arc<RwLock<HashMap<Address, ActorID>>>,
    id_to_addr: Arc<RwLock<HashMap<ActorID, Address>>>,
}

impl<C> AddressCache<C>
where
    C: Client + Sync + Send,
{
    pub fn new(client: FendermintClient<C>) -> Self {
        Self {
            client,
            addr_to_id: Default::default(),
            id_to_addr: Default::default(),
        }
    }

    pub async fn lookup_id(&self, addr: &Address) -> anyhow::Result<Option<ActorID>> {
        if let Ok(id) = addr.id() {
            return Ok(Some(id));
        }

        if let Some(id) = self.get_id(addr) {
            return Ok(Some(id));
        }

        let res = self
            .client
            .actor_state(addr, None)
            .await
            .context("failed to lookup actor state")?;

        match res.value {
            Some((id, _)) => {
                self.set_id(*addr, id);
                if let Payload::Delegated(_) = addr.payload() {
                    self.set_addr(id, *addr)
                }
                Ok(Some(id))
            }
            None => Ok(None),
        }
    }

    /// Look up the delegated address of an ID, if any.
    pub async fn lookup_addr(&self, id: &ActorID) -> anyhow::Result<Option<Address>> {
        if let Some(addr) = self.get_addr(id) {
            return Ok(Some(addr));
        }

        let res = self
            .client
            .actor_state(&Address::new_id(*id), None)
            .await
            .context("failed to lookup actor state")?;

        if let Some((_, actor_state)) = res.value {
            if let Some(addr) = actor_state.delegated_address {
                self.set_addr(*id, addr);
                self.set_id(addr, *id);
                return Ok(Some(addr));
            }
        }

        Ok(None)
    }

    fn get_id(&self, addr: &Address) -> Option<ActorID> {
        let c = self.addr_to_id.read().unwrap();
        c.get(addr).cloned()
    }

    fn set_id(&self, addr: Address, id: ActorID) {
        let mut c = self.addr_to_id.write().unwrap();
        c.insert(addr, id);
    }

    fn get_addr(&self, id: &ActorID) -> Option<Address> {
        let c = self.id_to_addr.read().unwrap();
        c.get(id).cloned()
    }

    fn set_addr(&self, id: ActorID, addr: Address) {
        let mut c = self.id_to_addr.write().unwrap();
        c.insert(id, addr);
    }
}
