// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! State Machine Test for the Staking contracts.
//!
//! The test simulates random actions validators can take, such as depositing and withdrawing
//! collateral, and executes these actions on the actual Solidity contracts as well as an
//! idealised model, comparing the results and testing that invariants are maintained.
//!
//! It can be executed the following way:
//!
//! ```text
//! cargo test --release -p contract-test --test staking
//! ```

#[test]
fn probe() {
    assert!(true, "It's working!")
}
