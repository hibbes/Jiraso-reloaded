// src-tauri/src/auth.rs
use crate::config::AppConfig;
use crate::error::AppResult;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rolle {
    Fachlehrer,
    Klassenlehrer,
    Administrator,
}

pub fn hash_password(pw: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon.hash_password(pw.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(pw: &str, encoded: &str) -> AppResult<bool> {
    if encoded.is_empty() {
        return Ok(false);
    }
    let parsed = PasswordHash::new(encoded)?;
    Ok(Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok())
}

/// Prüft das Passwort gegen alle drei Rollen-Hashes und gibt die erste
/// passende Rolle zurück. Reihenfolge: Admin > Klassenlehrer > Fachlehrer
/// (höhere Rolle gewinnt bei Kollision, damit ein Admin mit zufällig
/// identischem Passwort nicht auf Fachlehrer degradiert wird).
pub fn resolve_role(pw: &str, cfg: &AppConfig) -> AppResult<Option<Rolle>> {
    if verify_password(pw, &cfg.passwoerter.administrator)? {
        return Ok(Some(Rolle::Administrator));
    }
    if verify_password(pw, &cfg.passwoerter.klassenlehrer)? {
        return Ok(Some(Rolle::Klassenlehrer));
    }
    if verify_password(pw, &cfg.passwoerter.fachlehrer)? {
        return Ok(Some(Rolle::Fachlehrer));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_hash_verify() {
        let h = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }

    #[test]
    fn empty_hash_rejects() {
        assert!(!verify_password("any", "").unwrap());
    }

    #[test]
    fn resolve_role_prefers_highest() {
        let mut cfg = AppConfig::default();
        cfg.passwoerter.fachlehrer = hash_password("geheim").unwrap();
        cfg.passwoerter.klassenlehrer = hash_password("anders").unwrap();
        cfg.passwoerter.administrator = hash_password("geheim").unwrap();

        // "geheim" matcht Fachlehrer und Admin — Admin gewinnt
        assert_eq!(resolve_role("geheim", &cfg).unwrap(), Some(Rolle::Administrator));
        assert_eq!(resolve_role("anders", &cfg).unwrap(), Some(Rolle::Klassenlehrer));
        assert_eq!(resolve_role("unknown", &cfg).unwrap(), None);
    }
}
