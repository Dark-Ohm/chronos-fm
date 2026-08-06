//! S3 credentials manager backed by the system keyring (`oo7`), T011 Task 6.
//!
//! Stores access-key/secret-key pairs as a single JSON blob under
//! `chronos-fm-s3/<profile>`. Falls back gracefully: if the keyring is
//! unavailable, credentials must come from environment variables
//! (`AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`).

use anyhow::{Context, Result};
use oo7::Keyring;

/// Service identifier for S3 credential items.
const LABEL_PREFIX: &str = "Chronos-FM S3";

/// Manages S3 credential storage in the system keyring.
/// Each profile is stored as one item with a JSON blob containing
/// `access_key_id` and `secret_access_key`.
pub struct S3CredentialsManager {
    keyring: Option<Keyring>,
}

impl S3CredentialsManager {
    /// Connect to the system keyring. Returns `Ok(Some(..))` on success,
    /// or `Ok(None)` when no keyring backend is available.
    pub async fn connect() -> Result<Option<Self>> {
        match Keyring::new().await {
            Ok(keyring) => Ok(Some(Self {
                keyring: Some(keyring),
            })),
            Err(error) => {
                tracing::warn!(
                    "S3 keyring unavailable: {error}; \
                     credentials must be provided via environment variables"
                );
                Ok(None)
            }
        }
    }

    /// Store access key and secret for `profile`.
    pub async fn store(&self, profile: &str, access_key: &str, secret_key: &str) -> Result<()> {
        let keyring = self.keyring.as_ref().context("no keyring available")?;
        let label = format!("{LABEL_PREFIX} — {profile}");
        let json = format!(
            r#"{{"access_key_id":"{access_key}","secret_access_key":"{secret_key}"}}"#
        );
        // Remove any existing item for this profile first.
        self.delete(profile).await.ok();
        keyring
            .create_item(
                &label,
                &[("service", "chronos-fm-s3"), ("profile", profile)],
                oo7::Secret::from(json.as_str()),
                true,
            )
            .await
            .context("storing S3 credentials")?;
        Ok(())
    }

    /// Retrieve access key and secret for `profile`, or `None` if not stored.
    pub async fn retrieve(&self, profile: &str) -> Result<Option<(String, String)>> {
        let keyring = match &self.keyring {
            Some(k) => k,
            None => return Ok(None),
        };
        let items = keyring
            .search_items(&[("service", "chronos-fm-s3"), ("profile", profile)])
            .await
            .context("searching S3 credentials")?;
        let Some(item) = items.into_iter().next() else {
            return Ok(None);
        };
        let secret = item.secret().await.context("reading S3 secret")?;
        let json = String::from_utf8_lossy(secret.as_ref());
        let parsed: serde_json::Value =
            serde_json::from_str(&json).context("parsing stored credentials")?;
        let access = parsed["access_key_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let secret = parsed["secret_access_key"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if access.is_empty() || secret.is_empty() {
            return Ok(None);
        }
        Ok(Some((access, secret)))
    }

    /// Delete stored credentials for `profile`.
    pub async fn delete(&self, profile: &str) -> Result<()> {
        let keyring = match &self.keyring {
            Some(k) => k,
            None => return Ok(()),
        };
        let items = keyring
            .search_items(&[("service", "chronos-fm-s3"), ("profile", profile)])
            .await
            .context("searching for S3 credentials to delete")?;
        for item in items {
            item.delete().await.context("deleting S3 credential")?;
        }
        Ok(())
    }
}
