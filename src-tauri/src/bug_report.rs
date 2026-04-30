// src-tauri/src/bug_report.rs
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueResponse {
    pub number: i64,
    pub html_url: String,
}

/// Postet ein Issue ins konfigurierte GitHub-Repo.
/// Ohne Token wird ein sprechender Fehler zurückgegeben, das Frontend zeigt
/// dann einen mailto-Fallback.
pub fn submit(cfg: &AppConfig, titel: &str, body: &str) -> AppResult<IssueResponse> {
    if cfg.bug_report.github_token.trim().is_empty() {
        return Err(AppError::Config(
            "Kein GitHub-Token konfiguriert. Admin muss [bug_report].github_token in config.toml eintragen.".into(),
        ));
    }
    let url = format!("https://api.github.com/repos/{}/issues", cfg.bug_report.repo);
    let payload = serde_json::json!({
        "title": titel,
        "body": body,
        "labels": ["bug-report-app"],
    });

    // Proxy-Aufloesung: explizite Config gewinnt, sonst HTTPS_PROXY/HTTP_PROXY
    // aus Environment (Schul-Proxy ist haeufig dort gesetzt). reqwest 0.12
    // mit default-features=false aktiviert die Auto-Detection NICHT mehr,
    // deshalb explizit lesen.
    let proxy_url = if !cfg.bug_report.http_proxy.trim().is_empty() {
        Some(cfg.bug_report.http_proxy.trim().to_string())
    } else {
        std::env::var("HTTPS_PROXY")
            .or_else(|_| std::env::var("https_proxy"))
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("http_proxy"))
            .ok()
    };

    let mut builder = reqwest::blocking::Client::builder()
        .user_agent("jiraso-reloaded-bug-reporter")
        .timeout(std::time::Duration::from_secs(20));
    if let Some(p) = &proxy_url {
        let proxy = reqwest::Proxy::all(p)
            .map_err(|e| AppError::Config(format!("Proxy-URL ungueltig ({p}): {e}")))?;
        builder = builder.proxy(proxy);
    }
    let client = builder
        .build()
        .map_err(|e| AppError::Config(format!("HTTP-Client-Fehler: {e}")))?;

    let resp = client
        .post(&url)
        .bearer_auth(&cfg.bug_report.github_token)
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .map_err(|e| {
            // Volle Fehler-Kette ausgeben (TLS/DNS/Proxy-Details stecken in source).
            let mut chain = format!("{e}");
            let mut src: Option<&(dyn StdError + 'static)> = StdError::source(&e);
            while let Some(s) = src {
                chain.push_str(&format!(" -> {s}"));
                src = s.source();
            }
            let proxy_hint = match &proxy_url {
                Some(p) => format!(" (Proxy: {p})"),
                None => " (kein Proxy konfiguriert -- Schule? [bug_report].http_proxy in config.toml setzen)".into(),
            };
            AppError::Config(format!("GitHub-Request fehlgeschlagen{proxy_hint}: {chain}"))
        })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().unwrap_or_default();
        return Err(AppError::Config(format!("GitHub antwortete {status}: {txt}")));
    }
    let issue: IssueResponse = resp
        .json()
        .map_err(|e| AppError::Config(format!("GitHub-Antwort nicht parsebar: {e}")))?;
    Ok(issue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, BugReportConfig};

    #[test]
    fn submit_ohne_token_fehlt_aussagekraeftig() {
        let cfg = AppConfig {
            bug_report: BugReportConfig::default(),
            ..AppConfig::default()
        };
        let err = submit(&cfg, "x", "y").unwrap_err();
        assert!(format!("{err}").contains("Kein GitHub-Token"));
    }
}
