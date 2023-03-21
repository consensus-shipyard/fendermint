// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fendermint_testing::golden_json;
use fendermint_vm_genesis::Genesis;
use quickcheck::Arbitrary;

golden_json! { "genesis", genesis, Genesis::arbitrary }
