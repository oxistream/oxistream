//! Username and password policy, and argon2id hashing off the async runtime.

use std::sync::{Arc, OnceLock};

use argon2::password_hash::{PasswordHasher as _, PasswordVerifier as _};
use argon2::{Algorithm, Argon2, Params, Version};
use tokio::sync::Semaphore;

use crate::api::error::FieldError;

pub const USERNAME_MIN_LENGTH: usize = 3;
pub const USERNAME_MAX_LENGTH: usize = 32;
pub const PASSWORD_MIN_LENGTH: usize = 12;
pub const PASSWORD_MAX_LENGTH: usize = 256;

/// Letters, digits, dots, dashes and underscores; same rule as the frontend.
pub fn validate_username(username: &str) -> Result<(), FieldError> {
    if username.is_empty() {
        return Err(FieldError::Required);
    }
    let valid_chars = username
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !valid_chars || !(USERNAME_MIN_LENGTH..=USERNAME_MAX_LENGTH).contains(&username.len()) {
        return Err(FieldError::Format);
    }
    Ok(())
}

/// Length is counted in Unicode scalar values.
pub fn validate_password(password: &str) -> Result<(), FieldError> {
    let len = password.chars().count();
    if len == 0 {
        Err(FieldError::Required)
    } else if len < PASSWORD_MIN_LENGTH {
        Err(FieldError::Length)
    } else if len > PASSWORD_MAX_LENGTH {
        Err(FieldError::TooLong)
    } else {
        Ok(())
    }
}

/// OWASP's first recommended argon2id configuration: 19 MiB, 2 iterations, 1 lane.
fn argon2() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        argon2()
            .hash_password(b"oxistream-dummy-password")
            .expect("hash dummy password")
            .to_string()
    })
}

/// Hashes on the blocking pool, bounded so login floods cannot exhaust memory.
#[derive(Clone)]
pub struct PasswordHasher {
    permits: Arc<Semaphore>,
}

impl Default for PasswordHasher {
    fn default() -> Self {
        let parallelism = std::thread::available_parallelism().map_or(2, usize::from);
        Self {
            permits: Arc::new(Semaphore::new(parallelism)),
        }
    }
}

impl PasswordHasher {
    pub async fn hash(&self, password: String) -> anyhow::Result<String> {
        let _permit = self.permits.acquire().await?;
        tokio::task::spawn_blocking(move || {
            argon2()
                .hash_password(password.as_bytes())
                .map(|hash| hash.to_string())
                .map_err(|err| anyhow::anyhow!("hash password: {err}"))
        })
        .await?
    }

    /// Verifies against `hash`, or against a dummy hash when there is none so that unknown
    /// users take as long as known ones. Returns `false` in that case.
    pub async fn verify(&self, password: String, hash: Option<String>) -> anyhow::Result<bool> {
        let _permit = self.permits.acquire().await?;
        tokio::task::spawn_blocking(move || {
            let known = hash.is_some();
            let hash = hash.as_deref().unwrap_or_else(|| dummy_hash());
            let matches = argon2().verify_password(password.as_bytes(), hash).is_ok();
            known && matches
        })
        .await
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_rules() {
        for ok in ["abc", "mara.l-2", "Under_Score", &"x".repeat(32)] {
            assert_eq!(validate_username(ok), Ok(()), "{ok}");
        }
        assert_eq!(validate_username(""), Err(FieldError::Required));
        for bad in ["ab", "a b", "mära", "x@y", &"x".repeat(33)] {
            assert_eq!(validate_username(bad), Err(FieldError::Format), "{bad}");
        }
    }

    #[test]
    fn password_rules() {
        assert_eq!(validate_password(""), Err(FieldError::Required));
        assert_eq!(
            validate_password(&"x".repeat(PASSWORD_MIN_LENGTH - 1)),
            Err(FieldError::Length)
        );
        assert_eq!(validate_password(&"x".repeat(PASSWORD_MIN_LENGTH)), Ok(()));
        assert_eq!(validate_password(&"ä".repeat(PASSWORD_MAX_LENGTH)), Ok(()));
        assert_eq!(
            validate_password(&"x".repeat(PASSWORD_MAX_LENGTH + 1)),
            Err(FieldError::TooLong)
        );
        assert_eq!(validate_password(&"ä".repeat(6)), Err(FieldError::Length));
    }

    #[tokio::test]
    async fn hashes_and_verifies() {
        let hasher = PasswordHasher::default();
        let hash = hasher.hash("correct horse battery".into()).await.unwrap();
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(
            hasher
                .verify("correct horse battery".into(), Some(hash.clone()))
                .await
                .unwrap()
        );
        assert!(!hasher.verify("wrong".into(), Some(hash)).await.unwrap());
        assert!(
            !hasher
                .verify("oxistream-dummy-password".into(), None)
                .await
                .unwrap()
        );
    }
}
