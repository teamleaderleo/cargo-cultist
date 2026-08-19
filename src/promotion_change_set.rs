use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CHANGE_SET_ENTRIES: usize = 4096;
const MAX_PATH_BYTES: usize = 4096;
const GIT_SHA_HEX_BYTES: usize = 40;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionChangeSetEntry {
    pub path: String,
    pub blob_sha: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PromotionChangeSetError {
    message: String,
}

impl PromotionChangeSetError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PromotionChangeSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PromotionChangeSetError {}

pub fn fingerprint_promotion_change_set(
    entries: &[PromotionChangeSetEntry],
) -> Result<String, PromotionChangeSetError> {
    if entries.is_empty() || entries.len() > MAX_CHANGE_SET_ENTRIES {
        return Err(PromotionChangeSetError::new(
            "promotion change set must contain a bounded non-empty entry set",
        ));
    }

    let mut canonical = entries.to_vec();
    canonical.sort();
    let mut paths = BTreeSet::new();
    for entry in &canonical {
        validate_path(&entry.path)?;
        validate_blob_sha(&entry.blob_sha)?;
        if !paths.insert(entry.path.as_str()) {
            return Err(PromotionChangeSetError::new(format!(
                "duplicate promotion change-set path {}",
                entry.path
            )));
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(b"cultist-promotion-change-set-v1\0");
    for entry in &canonical {
        hasher.update(entry.path.as_bytes());
        hasher.update([0]);
        hasher.update(entry.blob_sha.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    Ok(format!("sha256:{hex}"))
}

fn validate_path(path: &str) -> Result<(), PromotionChangeSetError> {
    if path.is_empty()
        || path.trim() != path
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path.contains(['\n', '\r'])
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(PromotionChangeSetError::new(
            "promotion change-set paths must be canonical repository-relative paths",
        ));
    }
    Ok(())
}

fn validate_blob_sha(value: &str) -> Result<(), PromotionChangeSetError> {
    if value.len() != GIT_SHA_HEX_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PromotionChangeSetError::new(
            "promotion change-set blob_sha must be an exact lowercase 40-hex Git object id",
        ));
    }
    Ok(())
}
