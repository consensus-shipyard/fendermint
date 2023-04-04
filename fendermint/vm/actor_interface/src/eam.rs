// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fvm_shared::METHOD_CONSTRUCTOR;

define_singleton!(EAM {
    id: 10,
    code_id: 15
});

/// Ethereum Address Manager actor methods available.
#[repr(u64)]
pub enum Method {
    Constructor = METHOD_CONSTRUCTOR,
    Create = 2,
    Create2 = 3,
    CreateExternal = 4,
}
