// src-tauri/src/backup.rs
use crate::error::AppResult;
use chrono::{Datelike, NaiveDate, Utc};
use std::{fs, path::Path};

const RETENTION: usize = 30;

/// Erzeugt, falls heute noch nicht vorhanden, eine Kopie der DB als
/// data/backups/jiraso-YYYY-MM-DD.db. Rotiert auf 30 Dateien.
pub fn daily_backup(db: &Path, backups_dir: &Path) -> AppResult<Option<std::path::PathBuf>> {
    daily_backup_on(db, backups_dir, Utc::now().date_naive())
}

pub fn daily_backup_on(db: &Path, backups_dir: &Path, today: NaiveDate) -> AppResult<Option<std::path::PathBuf>> {
    if !db.exists() {
        return Ok(None);
    }
    fs::create_dir_all(backups_dir)?;
    let fname = format!(
        "jiraso-{:04}-{:02}-{:02}.db",
        today.year(),
        today.month(),
        today.day()
    );
    let target = backups_dir.join(&fname);

    if target.exists() {
        return Ok(None); // heute schon gesichert
    }

    fs::copy(db, &target)?;
    rotate(backups_dir, RETENTION)?;
    Ok(Some(target))
}

fn rotate(dir: &Path, keep: usize) -> AppResult<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|r| r.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("jiraso-")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    while entries.len() > keep {
        let oldest = entries.remove(0);
        fs::remove_file(oldest.path())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn no_db_means_no_backup() {
        let dir = tempdir().unwrap();
        let r = daily_backup(&dir.path().join("nope.db"), &dir.path().join("bk")).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn first_backup_creates_file() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"hello").unwrap();
        let bk = dir.path().join("bk");
        let r = daily_backup(&db, &bk).unwrap();
        assert!(r.is_some());
        assert_eq!(fs::read_dir(&bk).unwrap().count(), 1);
    }

    #[test]
    fn same_day_second_call_noop() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"hello").unwrap();
        let bk = dir.path().join("bk");
        daily_backup(&db, &bk).unwrap();
        let r2 = daily_backup(&db, &bk).unwrap();
        assert!(r2.is_none());
        assert_eq!(fs::read_dir(&bk).unwrap().count(), 1);
    }

    #[test]
    fn rotation_keeps_30() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"data").unwrap();
        let bk = dir.path().join("bk");

        // 32 Tage simulieren: Jan + Anfang Feb
        let mut d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        for _ in 0..32 {
            daily_backup_on(&db, &bk, d).unwrap();
            d = d.succ_opt().unwrap();
        }
        let count = fs::read_dir(&bk).unwrap().count();
        assert_eq!(count, 30);
    }
}
