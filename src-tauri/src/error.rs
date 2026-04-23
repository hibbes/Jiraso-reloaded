// src-tauri/src/error.rs
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Datenbankfehler: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Migrations-Fehler: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("Dateisystem-Fehler: {0}")]
    Io(#[from] std::io::Error),

    #[error("Konfigurations-Fehler: {0}")]
    Config(String),

    #[error("Passwort-Hash-Fehler: {0}")]
    Hash(String),

    #[error("Lock belegt: {0}")]
    Locked(String),

    #[error("TOML-Fehler: {0}")]
    Toml(String),
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AppError::Hash(e.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

// Fürs Frontend: String serialisieren
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
