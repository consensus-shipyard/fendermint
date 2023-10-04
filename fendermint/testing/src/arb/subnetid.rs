// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use fvm_shared::address::Address;
use ipc_sdk::subnet_id::SubnetID;

#[derive(Debug, Clone)]
pub struct ArbSubnetID(pub SubnetID);

impl quickcheck::Arbitrary for ArbSubnetID {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let child_count = usize::arbitrary(g) % 4;

        let children = (0..child_count)
            .map(|_| {
                if bool::arbitrary(g) {
                    Address::new_id(u64::arbitrary(g))
                } else {
                    // Only expecting EAM managed delegated addresses.
                    let subaddr: [u8; 20] = std::array::from_fn(|_| u8::arbitrary(g));
                    Address::new_delegated(10, &subaddr).unwrap()
                }
            })
            .collect::<Vec<_>>();

        Self(SubnetID::new(u64::arbitrary(g), children))
    }
}

impl arbitrary::Arbitrary<'_> for ArbSubnetID {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let child_count = usize::arbitrary(u)? % 4;

        let children = (0..child_count)
            .map(|_| {
                if bool::arbitrary(u)? {
                    Ok(Address::new_id(u64::arbitrary(u)?))
                } else {
                    // Only expecting EAM managed delegated addresses.
                    let mut subaddr = [0u8; 20];
                    for i in 0..20 {
                        subaddr[i] = u8::arbitrary(u)?;
                    }
                    Ok(Address::new_delegated(10, &subaddr).unwrap())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self(SubnetID::new(u64::arbitrary(u)?, children)))
    }
}
