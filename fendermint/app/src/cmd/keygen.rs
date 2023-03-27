// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::io::Write;
use std::{fs::File, path::Path};

use base64::Engine;
use libsecp256k1::{PublicKey, SecretKey};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

use crate::{cmd, options::KeygenArgs};

cmd! {
  KeygenArgs(self) {
    let mut rng = ChaCha20Rng::from_entropy();
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from_secret_key(&sk);

    export(&self.out_dir, &self.name, "sk", &secret_to_b64(&sk))?;
    export(&self.out_dir, &self.name, "pk", &public_to_b64(&pk))?;

    Ok(())
  }
}

/// Encode bytes in a format that the Genesis deserializer can handle.
fn to_b64(bz: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(bz)
}

fn secret_to_b64(sk: &SecretKey) -> String {
    to_b64(&sk.serialize())
}

fn public_to_b64(pk: &PublicKey) -> String {
    to_b64(&pk.serialize_compressed())
}

fn export(output_dir: &Path, name: &str, ext: &str, b64: &str) -> anyhow::Result<()> {
    let output_path = output_dir.join(format!("{name}.{ext}"));
    let mut output = File::create(output_path)?;
    write!(&mut output, "{}", b64)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use fendermint_vm_genesis::ValidatorKey;
    use libsecp256k1::PublicKey;
    use quickcheck_macros::quickcheck;

    use super::public_to_b64;

    #[quickcheck]
    fn prop_public_key_deserialize_to_genesis(vk: ValidatorKey) {
        let b64 = public_to_b64(&vk.0);
        let json = serde_json::json!(b64);
        let pk: PublicKey = serde_json::from_value(json).unwrap();
        assert_eq!(pk, vk.0)
    }
}
