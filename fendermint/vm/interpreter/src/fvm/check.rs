// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use async_trait::async_trait;

use fvm_ipld_blockstore::Blockstore;
use fvm_shared::{address::Address, error::ExitCode};

use crate::CheckInterpreter;

use super::{state::FvmCheckState, FvmMessage, FvmMessageInterpreter};

/// Transaction check results are expressed by the exit code, so that hopefully
/// they would result in the same error code if they were applied.
pub struct FvmCheckRet {
    pub sender: Address,
    pub gas_limit: u64,
    pub exit_code: ExitCode,
    pub info: Option<String>,
}

#[async_trait]
impl<DB> CheckInterpreter for FvmMessageInterpreter<DB>
where
    DB: Blockstore + 'static + Send + Sync,
{
    type State = FvmCheckState<DB>;
    type Message = FvmMessage;
    type Output = FvmCheckRet;

    /// Check that:
    /// * sender exists
    /// * sender nonce matches the message sequence
    /// * sender has enough funds to cover the gas cost
    async fn check(
        &self,
        mut state: Self::State,
        msg: Self::Message,
        _is_recheck: bool,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        let checked = |state, exit_code: ExitCode, info: Option<String>| {
            if !exit_code.is_success() {
                tracing::info!(
                    exit_code = exit_code.value(),
                    from = msg.from.to_string(),
                    to = msg.to.to_string(),
                    method_num = msg.method_num,
                    info = info.clone().unwrap_or_default(),
                    "check failed"
                );
            }
            let ret = FvmCheckRet {
                sender: msg.from,
                gas_limit: msg.gas_limit,
                exit_code,
                info,
            };
            Ok((state, ret))
        };

        if let Err(e) = msg.check() {
            return checked(
                state,
                ExitCode::SYS_ASSERTION_FAILED,
                Some(format!("pre-check failure: {:#}", e)),
            );
        }

        // NOTE: This would be a great place for let-else, but clippy runs into a compilation bug.
        let state_tree = state.state_tree_mut();

        if let Some(id) = state_tree.lookup_id(&msg.from)? {
            if let Some(mut actor) = state_tree.get_actor(id)? {
                let balance_needed = msg.gas_fee_cap * msg.gas_limit;
                if actor.balance < balance_needed {
                    return checked(
                        state,
                        ExitCode::SYS_SENDER_STATE_INVALID,
                        Some(
                            format! {"actor balance {} less than needed {}", actor.balance, balance_needed},
                        ),
                    );
                } else if actor.sequence != msg.sequence {
                    return checked(
                        state,
                        ExitCode::SYS_SENDER_STATE_INVALID,
                        Some(
                            format! {"expected sequence {}, got {}", actor.sequence, msg.sequence},
                        ),
                    );
                } else {
                    actor.sequence += 1;
                    actor.balance -= balance_needed;
                    state_tree.set_actor(id, actor);
                    return checked(state, ExitCode::OK, None);
                }
            }
        }

        checked(
            state,
            ExitCode::SYS_SENDER_INVALID,
            Some(format! {"cannot find actor {}", msg.from}),
        )
    }
}
