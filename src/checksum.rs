//! Content hashing. `olivaw.toml` records `sha256:<hex>` of every vendored
//! file at install time; `update`/`check` compare against it to detect user
//! edits. This format is stable — changing it invalidates every recorded
//! manifest.

use sha2::{Digest, Sha256};

/// `sha256:<lowercase hex>` of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for b in digest {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_is_stable() {
        // Known SHA-256 of the empty string.
        assert_eq!(
            sha256_hex(b""),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex(b"olivaw"), sha256_hex(b"olivaw"),);
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
    }
}
