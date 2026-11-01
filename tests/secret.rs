use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ddd_toolkit_core::{Secret, SecretMetadata};
use zeroize::Zeroize;

struct Probe {
    value: Vec<u8>,
    zeroized: Arc<AtomicBool>,
}

impl Zeroize for Probe {
    fn zeroize(&mut self) {
        self.value.zeroize();
        self.zeroized.store(true, Ordering::SeqCst);
    }
}

#[test]
fn secret_redacts_and_exposes_only_explicitly() {
    let secret = Secret::new(String::from("top-secret"));
    assert_eq!(format!("{:?}", secret), "[REDACTED]");
    assert_eq!(secret.to_string(), "[REDACTED]");
    assert_eq!(secret.as_inner(), "top-secret");
    assert_eq!(secret.expose_secret(|value| value.len()), 10);
    assert_eq!(secret.into_inner(), "top-secret");
}

#[test]
fn secret_zeroizes_on_drop() {
    let zeroized = Arc::new(AtomicBool::new(false));
    {
        let _secret = Secret::new(Probe {
            value: b"secret".to_vec(),
            zeroized: Arc::clone(&zeroized),
        });
    }
    assert!(zeroized.load(Ordering::SeqCst));
}

#[test]
fn secret_clone_and_equality_delegate_to_inner() {
    let left = Secret::new(String::from("same"));
    let right = left.clone();
    assert_eq!(left, right);
    assert_eq!(right.as_inner(), "same");
}

#[test]
fn secret_metadata_is_zero_sized() {
    assert_eq!(std::mem::size_of::<SecretMetadata>(), 0);
    assert_eq!(SecretMetadata, SecretMetadata);
}
