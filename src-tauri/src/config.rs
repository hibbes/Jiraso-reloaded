// src-tauri/src/config.rs
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub schule: String,
    pub aktuelles_schuljahr: String,
    pub passwoerter: Passwoerter,
    pub goodies: GoodiesConfig,
    pub lock: LockConfig,
    #[serde(default)]
    pub bug_report: BugReportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BugReportConfig {
    #[serde(default)]
    pub github_token: String,
    #[serde(default = "default_repo")]
    pub repo: String,
    /// Optionaler HTTPS-Proxy fuer api.github.com (Schul-Proxy).
    /// Beispiele: "http://proxy.schule.lan:8080" oder
    /// "http://user:pass@proxy.schule.lan:8080".
    /// Leer = kein Proxy bzw. HTTPS_PROXY/HTTP_PROXY-Env-Var nutzen.
    #[serde(default)]
    pub http_proxy: String,
}

fn default_repo() -> String {
    "hibbes/Jiraso-reloaded".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Passwoerter {
    pub fachlehrer: String,
    pub klassenlehrer: String,
    pub administrator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodiesConfig {
    pub enabled: bool,
    pub mode: String,
}

impl Default for GoodiesConfig {
    fn default() -> Self {
        Self { enabled: true, mode: "random".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockConfig {
    pub stale_hours: u32,
    pub max_parallel: u32,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self { stale_hours: 8, max_parallel: 2 }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schule: "Schiller-Gymnasium Offenburg".into(),
            aktuelles_schuljahr: "2025/26".into(),
            passwoerter: Passwoerter::default(),
            goodies: GoodiesConfig::default(),
            lock: LockConfig::default(),
            bug_report: BugReportConfig {
                http_proxy: String::new(),
                github_token: String::new(),
                repo: default_repo(),
            },
        }
    }
}

pub fn load_or_create(path: &Path) -> AppResult<AppConfig> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let cfg: AppConfig = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        let cfg = AppConfig::default();
        save(&cfg, path)?;
        Ok(cfg)
    }
}

pub fn save(cfg: &AppConfig, path: &Path) -> AppResult<()> {
    let content = toml::to_string_pretty(cfg)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn passwoerter_leer(cfg: &AppConfig) -> bool {
    cfg.passwoerter.fachlehrer.is_empty()
        && cfg.passwoerter.klassenlehrer.is_empty()
        && cfg.passwoerter.administrator.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_default_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = load_or_create(&path).unwrap();
        assert!(path.exists());
        assert_eq!(cfg.lock.max_parallel, 2);
        assert!(passwoerter_leer(&cfg));
    }

    #[test]
    fn reads_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "schule = \"Testschule\"\naktuelles_schuljahr = \"2020/21\"\n\n[passwoerter]\nfachlehrer = \"h1\"\nklassenlehrer = \"h2\"\nadministrator = \"h3\"\n\n[goodies]\nenabled = false\nmode = \"zitat_only\"\n\n[lock]\nstale_hours = 4\nmax_parallel = 2\n").unwrap();
        let cfg = load_or_create(&path).unwrap();
        assert_eq!(cfg.schule, "Testschule");
        assert!(!cfg.goodies.enabled);
        assert_eq!(cfg.lock.stale_hours, 4);
    }
}
