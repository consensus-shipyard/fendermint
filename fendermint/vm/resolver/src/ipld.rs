// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::time::Duration;

use async_stm::{atomically, queues::TQueueLike};
use ipc_ipld_resolver::Client as ResolverClient;

use crate::pool::{ResolveQueue, ResolveTask};

/// The IPLD Resolver takes resolution tasks from the [ResolvePool] and
/// uses the [ipc_ipld_resolver] to fetch the content from subnets.
pub struct IpldResolver {
    client: ResolverClient,
    queue: ResolveQueue,
    retry_delay: Duration,
}

impl IpldResolver {
    pub fn new(client: ResolverClient, queue: ResolveQueue, retry_delay: Duration) -> Self {
        Self {
            client,
            queue,
            retry_delay,
        }
    }

    /// Start taking tasks from the resolver pool and resolving them using the IPLD Resolver.
    pub async fn run(self) {
        loop {
            let task = atomically(|| self.queue.read()).await;
            // TODO: Is it okay to wait for the resolution of one item before trying the next one?
            //       There are items coming from all child subnets, so in theory we could resolve
            //       them in parallel, but in practice we should probably limit how many outstanding
            //       tasks we have.
            // TODO: When do we try to resolve from our own subnet, our own peers?
            match self.client.resolve(task.cid(), task.subnet_id()).await {
                Err(e) => {
                    tracing::error!(error = e.to_string(), "failed to submit resolution task");
                    // The service is no longer listening, we might as well quit.
                    // By not quitting we should see this error every time there is a new task.
                    // We could send some feedback, but it's really fatal.
                }
                Ok(result) => match result {
                    Ok(()) => {
                        tracing::debug!(cid = ?task.cid(), "content resolved");
                        atomically(|| task.set_resolved()).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            cid = ?task.cid(),
                            error = e.to_string(),
                            "content resolution failed; retrying later"
                        );
                        self.retry_later(task);
                    }
                },
            }
        }
    }

    /// Part of error handling.
    ///
    /// In our case we enqueued the task from transaction processing,
    /// which will not happen again, so there is no point further
    /// propagating this error back to the sender to deal with.
    /// Rather, we should retry until we can conclude whether it will
    /// ever complete. Some errors raised by the service are transitive,
    /// such as having no peers currently, but that might change.
    ///
    /// For now, let's retry the same task later.
    fn retry_later(&self, task: ResolveTask) {
        let retry_delay = self.retry_delay;
        let queue = self.queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(retry_delay).await;
            tracing::debug!(cid = ?task.cid(), "retrying content resolution");
            atomically(move || queue.write(task.clone())).await;
        });
    }
}
