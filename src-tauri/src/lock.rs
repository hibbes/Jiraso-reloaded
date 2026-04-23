// src-tauri/src/lock.rs
use crate::error::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub rechner: String,
    pub rolle: String,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LockFile {
    sessions: Vec<Session>,
}

pub struct LockConfig {
    pub stale_hours: u32,
    pub max_parallel: u32,
}

fn gen_session_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{:08x}", rng.gen::<u32>())
}

fn read_file(path: &Path) -> AppResult<LockFile> {
    if !path.exists() {
        return Ok(LockFile::default());
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn write_file(path: &Path, lf: &LockFile) -> AppResult<()> {
    let content = serde_json::to_string_pretty(lf)
        .map_err(|e| AppError::Config(e.to_string()))?;
    fs::write(path, content)?;
    Ok(())
}

fn prune_stale(lf: &mut LockFile, stale_hours: u32, now: DateTime<Utc>) {
    let cutoff = now - Duration::hours(stale_hours as i64);
    lf.sessions.retain(|s| s.since > cutoff);
}

/// Versucht, eine Session zu registrieren. Gibt Session oder Locked-Fehler zurück.
pub fn acquire(
    path: &Path,
    rechner: &str,
    rolle: &str,
    cfg: &LockConfig,
) -> AppResult<Session> {
    acquire_at(path, rechner, rolle, cfg, Utc::now())
}

// separater Entry-Point, damit Tests die Uhr stellen können
pub fn acquire_at(
    path: &Path,
    rechner: &str,
    rolle: &str,
    cfg: &LockConfig,
    now: DateTime<Utc>,
) -> AppResult<Session> {
    let mut lf = read_file(path)?;
    prune_stale(&mut lf, cfg.stale_hours, now);

    if lf.sessions.len() >= cfg.max_parallel as usize {
        let names: Vec<String> = lf
            .sessions
            .iter()
            .map(|s| format!("{} ({}, seit {})", s.rechner, s.rolle, s.since.format("%H:%M")))
            .collect();
        return Err(AppError::Locked(format!(
            "Aktuell sind {} angemeldet. Bitte warten, bis jemand das Programm schließt.",
            names.join(" und ")
        )));
    }

    let session = Session {
        id: gen_session_id(),
        rechner: rechner.into(),
        rolle: rolle.into(),
        since: now,
    };
    lf.sessions.push(session.clone());
    write_file(path, &lf)?;
    Ok(session)
}

pub fn release(path: &Path, session_id: &str) -> AppResult<()> {
    let mut lf = read_file(path)?;
    lf.sessions.retain(|s| s.id != session_id);
    write_file(path, &lf)?;
    Ok(())
}

/// Admin-Override: entfernt ALLE Sessions.
pub fn break_lock(path: &Path) -> AppResult<()> {
    let lf = LockFile::default();
    write_file(path, &lf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cfg() -> LockConfig {
        LockConfig { stale_hours: 8, max_parallel: 2 }
    }

    #[test]
    fn acquire_succeeds_when_empty() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let s = acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        assert_eq!(s.rechner, "PC-A");
    }

    #[test]
    fn second_session_ok_third_blocked() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Klassenlehrer", &cfg()).unwrap();
        let e = acquire(&p, "PC-C", "Fachlehrer", &cfg());
        assert!(matches!(e, Err(AppError::Locked(_))));
    }

    #[test]
    fn stale_sessions_are_pruned() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let old = Utc::now() - Duration::hours(9);
        acquire_at(&p, "PC-A", "Fachlehrer", &cfg(), old).unwrap();
        acquire_at(&p, "PC-B", "Fachlehrer", &cfg(), old).unwrap();
        // beide stale -> neuer Slot frei
        let now = Utc::now();
        let s = acquire_at(&p, "PC-C", "Fachlehrer", &cfg(), now).unwrap();
        assert_eq!(s.rechner, "PC-C");

        let lf = read_file(&p).unwrap();
        assert_eq!(lf.sessions.len(), 1);
    }

    #[test]
    fn release_frees_slot() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let a = acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Fachlehrer", &cfg()).unwrap();
        release(&p, &a.id).unwrap();
        // Slot frei
        acquire(&p, "PC-C", "Fachlehrer", &cfg()).unwrap();
    }

    #[test]
    fn break_lock_clears_all() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Fachlehrer", &cfg()).unwrap();
        break_lock(&p).unwrap();
        // wieder beide Slots frei
        acquire(&p, "PC-C", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-D", "Fachlehrer", &cfg()).unwrap();
    }
}
