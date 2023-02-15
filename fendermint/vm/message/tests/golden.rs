use cid::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Path to a golden file.
fn path(prefix: &str, name: &str, ext: &str) -> String {
    // All files will have the same name but different extension.
    // They should be under `fendermint/vm/message/golden`.
    let path = Path::new("golden").join(prefix).join(name);
    format!("{}.{}", path.display(), ext)
}

/// Read the contents of an existing golden file, or create it by turning `fallback` into string first.
fn read_or_create<T>(
    prefix: &str,
    name: &str,
    ext: &str,
    fallback: &T,
    to_string: fn(&T) -> String,
) -> String {
    let p = path(prefix, name, ext);
    let p = Path::new(&p);

    if !p.exists() {
        if let Some(p) = p.parent() {
            std::fs::create_dir_all(p).expect("failed to create golden directory");
        }
        let s = to_string(fallback);
        let mut f = File::create(&p)
            .unwrap_or_else(|e| panic!("Cannot create golden file at {:?}: {}", p, e));
        f.write_all(s.as_bytes()).unwrap();
    }

    let mut f =
        File::open(&p).unwrap_or_else(|e| panic!("Cannot open golden file at {:?}: {}", p, e));

    let mut s = String::new();
    f.read_to_string(&mut s).expect("Cannot read golden file.");
    s
}

/// Check that a golden file we created earlier can still be read by the current model by
/// comparing to a debug string (which should at least be readable enough to show what changed).
///
/// If the golden file doesn't exist, create one now.
fn test_cbor_txt<T: Serialize + DeserializeOwned + Debug>(
    prefix: &str,
    name: &str,
    arb_data: fn(g: &mut quickcheck::Gen) -> T,
) -> T {
    // We may not need this, but it shouldn't be too expensive to generate.
    let mut g = quickcheck::Gen::new(10);
    let data0 = arb_data(&mut g);

    // Debug string of a wrapper.
    let to_debug = |w: &T| format!("{:?}", w);

    let cbor = read_or_create(prefix, name, "cbor", &data0, |d| {
        let bz = fvm_ipld_encoding::to_vec(d).expect("failed to serialize");
        hex::encode(bz)
    });

    let bz = hex::decode(cbor).expect("failed to decode hex");
    let data1: T = fvm_ipld_encoding::from_slice(&bz)
        .expect(&format!("Cannot deserialize {}/{}.cbor", prefix, name));

    // Use the deserialised data as fallback for the debug string, so if the txt doesn't exist, it's created
    // from what we just read back.
    let txt = read_or_create(prefix, name, "txt", &data0, to_debug);

    // This will fail if either the CBOR or the Debug format changes.
    // At that point we should either know that it's a legitimate regression because we changed the model,
    // or catch it as an unexpected regression, indicating that we made some backwards incompatible change.
    assert_eq!(to_debug(&data1), txt.trim_end());

    data1
}

/// Test that the CID of something we deserialized from CBOR matches what we saved earlier,
/// ie. that we produce the same CID, which is important if it's the basis of signing.
pub fn test_cid<T: Debug>(prefix: &str, name: &str, data: T, cid: fn(&T) -> Cid) {
    let exp_cid = cid(&data);
    let hex_cid = read_or_create(prefix, name, "cid", &exp_cid, |d| hex::encode(d.to_bytes()));
    let exp_cid = hex::encode(exp_cid.to_bytes());
    assert_eq!(hex_cid, exp_cid)
}

macro_rules! golden_cbor {
    ($prefix:literal, $name:ident, $gen:expr) => {
        #[test]
        fn $name() {
            let label = stringify!($name);
            crate::test_cbor_txt($prefix, &label, $gen);
        }
    };
}

macro_rules! golden_cid {
    ($prefix:literal, $name:ident, $gen:expr, $cid:expr) => {
        #[test]
        fn $name() {
            let label = stringify!($name);
            let data = crate::test_cbor_txt($prefix, &label, $gen);
            crate::test_cid($prefix, &label, data, $cid);
        }
    };
}

/// Examples of `ChainMessage`.
mod chain {
    use fendermint_vm_message::chain::ChainMessage;
    use quickcheck::Arbitrary;

    golden_cbor! { "chain", signed, |g| {
        loop {
          if let msg @ ChainMessage::Signed(_) = ChainMessage::arbitrary(g) {
            return msg
          }
        }
      }
    }

    golden_cbor! { "chain", for_execution, |g| {
        loop {
          if let msg @ ChainMessage::ForExecution(_) = ChainMessage::arbitrary(g) {
            return msg
          }
        }
      }
    }

    golden_cbor! { "chain", for_resolution, |g| {
        loop {
          if let msg @ ChainMessage::ForResolution(_) = ChainMessage::arbitrary(g) {
            return msg
          }
        }
      }
    }
}

/// Examples of FVM messages, which is what we sign.
mod fvm {
    use fendermint_vm_message::signed::SignedMessage;
    use quickcheck::Arbitrary;

    golden_cid! { "fvm", message, |g| SignedMessage::arbitrary(g).message, |m| SignedMessage::cid(m).unwrap() }
}
