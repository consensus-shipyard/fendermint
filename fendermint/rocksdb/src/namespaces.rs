// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

/// List all column families to help keep them unique.
///
/// # Example
///
/// ```
/// namespaces!(MySpace { foo, bar })
/// ```
#[macro_export]
macro_rules! namespaces {
    ($name:ident { $($col:ident),* }) => {
        struct $name {
            pub $($col: String),+
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $($col: stringify!($col).to_owned()),+
                }
            }
        }

        impl $name {
            /// List column family names, all of which are required for re-opening the databasae.
            pub fn values(&self) -> Vec<&str> {
                vec![$(self.$col.as_ref()),+]
            }
        }
    };
}
