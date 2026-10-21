// src/client/browser_cookies.rs
//! Browser cookie reading functionality for automatic SESSDATA detection

use chrono::{DateTime, TimeZone, Utc};
use directories::UserDirs;
use log::{debug, info, warn};
use sqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<DateTime<Utc>>,
    pub secure: bool,
    pub http_only: bool,
}

#[derive(Debug)]
pub enum Browser {
    Chrome,
    Firefox,
    Edge,
    Chromium,
    Opera,
}

impl Browser {
    pub fn get_cookie_db_path(&self) -> Option<PathBuf> {
        let user_dirs = UserDirs::new()?;
        let home_dir = user_dirs.home_dir();

        match self {
            Browser::Chrome => {
                #[cfg(target_os = "linux")]
                {
                    Some(home_dir.join(".config/google-chrome/Default/Cookies"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home_dir.join("Library/Application Support/Google/Chrome/Default/Cookies"))
                }
                #[cfg(target_os = "windows")]
                {
                    Some(
                        home_dir
                            .join("AppData/Local/Google/Chrome/User Data/Default/Network/Cookies"),
                    )
                }
            }
            Browser::Firefox => {
                #[cfg(target_os = "linux")]
                {
                    let firefox_dir = vec![
                        home_dir.join(".mozilla/firefox"),
                        home_dir.join("snap/firefox/common/.mozilla/firefox"),
                        home_dir.join(".var/app/org.mozilla.firefox/.mozilla/firefox"),
                        home_dir.join("snap/firefox/current/.mozilla/firefox"),
                    ]
                    .into_iter()
                    .find(|p| p.exists())?;

                    Self::find_firefox_profile_cookies(&firefox_dir)
                }
                #[cfg(target_os = "macos")]
                {
                    let firefox_dir = home_dir.join("Library/Application Support/Firefox/Profiles");
                    Self::find_firefox_profile_cookies(&firefox_dir)
                }
                #[cfg(target_os = "windows")]
                {
                    let firefox_dir = home_dir.join("AppData/Roaming/Mozilla/Firefox/Profiles");
                    Self::find_firefox_profile_cookies(&firefox_dir)
                }
            }
            Browser::Edge => {
                #[cfg(target_os = "linux")]
                {
                    Some(home_dir.join(".config/microsoft-edge/Default/Cookies"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(
                        home_dir.join("Library/Application Support/Microsoft Edge/Default/Cookies"),
                    )
                }
                #[cfg(target_os = "windows")]
                {
                    Some(
                        home_dir
                            .join("AppData/Local/Microsoft/Edge/User Data/Default/Network/Cookies"),
                    )
                }
            }
            Browser::Chromium => {
                #[cfg(target_os = "linux")]
                {
                    Some(home_dir.join(".config/chromium/Default/Cookies"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home_dir.join("Library/Application Support/Chromium/Default/Cookies"))
                }
                #[cfg(target_os = "windows")]
                {
                    Some(home_dir.join("AppData/Local/Chromium/User Data/Default/Network/Cookies"))
                }
            }
            Browser::Opera => {
                #[cfg(target_os = "linux")]
                {
                    Some(home_dir.join(".config/opera/Default/Cookies"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home_dir.join(
                        "Library/Application Support/com.operasoftware.Opera/Default/Cookies",
                    ))
                }
                #[cfg(target_os = "windows")]
                {
                    Some(
                        home_dir
                            .join("AppData/Roaming/Opera Software/Opera Stable/Network/Cookies"),
                    )
                }
            }
        }
    }

    fn find_firefox_profile_cookies(firefox_dir: &Path) -> Option<PathBuf> {
        if !firefox_dir.exists() {
            return None;
        }

        // Look for the default profile directory
        let entries = fs::read_dir(firefox_dir).ok()?;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name()?.to_str()?;
                    if dir_name.contains(".default") || dir_name.contains(".default-release") {
                        let cookies_path = path.join("cookies.sqlite");
                        if cookies_path.exists() {
                            return Some(cookies_path);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_all_supported() -> Vec<Browser> {
        vec![
            Browser::Chrome,
            Browser::Firefox,
            Browser::Edge,
            Browser::Chromium,
            Browser::Opera,
        ]
    }
}

/// Read cookies from a browser's cookie database
pub fn read_cookies_from_browser(
    browser: &Browser,
    domain_filter: Option<&str>,
) -> Result<Vec<Cookie>, String> {
    let db_path = browser
        .get_cookie_db_path()
        .ok_or_else(|| "Could not determine cookie database path".to_string())?;

    if !db_path.exists() {
        return Err(format!("Cookie database not found at: {:?}", db_path));
    }

    debug!("Reading cookies from: {:?}", db_path);

    // Create a temporary copy of the database since browsers might have it locked
    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Failed to create private cookie database copy: {}", e))?;
    let temp_path = temp_file.path();
    let mut source =
        fs::File::open(&db_path).map_err(|e| format!("Failed to open cookie database: {}", e))?;
    std::io::copy(&mut source, &mut temp_file.as_file())
        .map_err(|e| format!("Failed to copy cookie database: {}", e))?;

    let result = match browser {
        Browser::Firefox => read_firefox_cookies(temp_path, domain_filter),
        _ => read_chromium_cookies(temp_path, domain_filter, browser),
    };

    // The private temporary copy is removed on drop, including error paths.
    result
}

fn read_chromium_cookies(
    db_path: &Path,
    domain_filter: Option<&str>,
    _browser: &Browser,
) -> Result<Vec<Cookie>, String> {
    let connection =
        Connection::open(db_path).map_err(|e| format!("Failed to open cookie database: {}", e))?;

    let mut statement = connection.prepare(
        "SELECT name, value, host_key, path, expires_utc, is_secure, is_httponly, encrypted_value
         FROM cookies WHERE (?1 IS NULL OR host_key = ?1 OR host_key LIKE ?2)",
    ).map_err(|e| format!("Failed to prepare cookie query: {}", e))?;
    let suffix = domain_filter.map(|domain| format!("%.{}", domain));
    statement
        .bind((1, domain_filter))
        .map_err(|e| e.to_string())?;
    statement
        .bind((2, suffix.as_deref()))
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    let version: u32 = {
        let mut meta = connection
            .prepare("SELECT value FROM meta WHERE key = 'version'")
            .map_err(|e| format!("Could not read Chromium database version: {}", e))?;
        if meta.next().map_err(|e| e.to_string())? != sqlite::State::Row {
            return Err("Missing Chromium database version".into());
        }
        meta.read::<String, _>(0)
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|_| "Invalid Chromium database version".to_string())?
    };
    #[cfg(target_os = "linux")]
    let mut key = None;
    let mut cookies = Vec::new();
    while statement.next().map_err(|e| e.to_string())? == sqlite::State::Row {
        let name = statement.read::<String, _>(0).map_err(|e| e.to_string())?;
        let value = statement.read::<String, _>(1).map_err(|e| e.to_string())?;
        let domain = statement.read::<String, _>(2).map_err(|e| e.to_string())?;
        let encrypted = statement.read::<Vec<u8>, _>(7).map_err(|e| e.to_string())?;
        let value = if value.is_empty() && !encrypted.is_empty() {
            #[cfg(target_os = "linux")]
            {
                let key_bytes = if encrypted.starts_with(b"v11") {
                    match key.get_or_insert_with(|| linux::keyring_key(_browser)) {
                        Ok(bytes) => Some(bytes.as_slice()),
                        Err(e) => return Err(e.clone()),
                    }
                } else {
                    None
                };
                match linux::decrypt(&encrypted, &domain, version, key_bytes) {
                    Ok(value) => value,
                    Err(e) => {
                        warn!("Skipping encrypted cookie {}: {}", name, e);
                        continue;
                    }
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                warn!(
                    "Skipping encrypted cookie {}: decryption is only supported on Linux",
                    name
                );
                continue;
            }
        } else {
            value
        };
        if value.is_empty() {
            continue;
        }
        let expiry = statement.read::<i64, _>(4).map_err(|e| e.to_string())?;
        cookies.push(Cookie {
            name,
            value,
            domain,
            path: statement.read::<String, _>(3).map_err(|e| e.to_string())?,
            expires: if expiry == 0 {
                None
            } else {
                Utc.timestamp_opt(expiry / 1_000_000 - 11644473600, 0)
                    .single()
            },
            secure: statement.read::<i64, _>(5).map_err(|e| e.to_string())? != 0,
            http_only: statement.read::<i64, _>(6).map_err(|e| e.to_string())? != 0,
        });
    }
    Ok(cookies)
}

fn read_firefox_cookies(
    db_path: &Path,
    domain_filter: Option<&str>,
) -> Result<Vec<Cookie>, String> {
    let connection =
        Connection::open(db_path).map_err(|e| format!("Failed to open cookie database: {}", e))?;

    let mut query =
        "SELECT name, value, host, path, expiry, isSecure, isHttpOnly FROM moz_cookies".to_string();

    if let Some(domain) = domain_filter {
        query.push_str(&format!(" WHERE host LIKE '%{}'", domain));
    }

    let mut cookies = Vec::new();

    connection
        .iterate(query, |pairs| {
            let mut cookie_data = HashMap::new();
            for &(column, value) in pairs.iter() {
                cookie_data.insert(column, value.unwrap_or(""));
            }

            let expires = if let Some(expires_str) = cookie_data.get("expiry") {
                if let Ok(expires_timestamp) = expires_str.parse::<i64>() {
                    Utc.timestamp_opt(expires_timestamp, 0).single()
                } else {
                    None
                }
            } else {
                None
            };

            let cookie = Cookie {
                name: cookie_data.get("name").unwrap_or(&"").to_string(),
                value: cookie_data.get("value").unwrap_or(&"").to_string(),
                domain: cookie_data.get("host").unwrap_or(&"").to_string(),
                path: cookie_data.get("path").unwrap_or(&"").to_string(),
                expires,
                secure: cookie_data.get("isSecure").unwrap_or(&"0") == &"1",
                http_only: cookie_data.get("isHttpOnly").unwrap_or(&"0") == &"1",
            };

            cookies.push(cookie);
            true
        })
        .map_err(|e| format!("Failed to query cookies: {}", e))?;

    Ok(cookies)
}

/// Select the first API-verified browser session, never mixing accounts.
pub fn find_bilibili_cookies_as_string() -> Option<String> {
    match find_verified_bilibili_cookies() {
        Ok(cookies) => Some(cookies),
        Err(reason) => {
            warn!("{}", reason);
            None
        }
    }
}

/// Fallible auto-detection retaining validation failures for startup diagnostics.
pub fn find_verified_bilibili_cookies() -> Result<String, String> {
    // Expiration dates cannot detect server-side logout. Verify login with a
    // read-only request before choosing credentials for receiving and sending.
    let client = match reqwest::blocking::Client::builder()
        .https_only(true)
        .user_agent(crate::auth::USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return Err(
                "Could not initialize browser login validation; no session selected".into(),
            );
        }
    };
    let mut validation_failed = false;
    for browser in Browser::get_all_supported() {
        info!("Checking browser: {:?}", browser);
        let cookies = match read_cookies_from_browser(&browser, Some("bilibili.com")) {
            Ok(cookies) => cookies,
            Err(e) => {
                debug!("Could not read {:?} cookies: {}", browser, e);
                continue;
            }
        };
        if let Some(cookie_string) = select_browser_cookies(cookies) {
            match validate_browser_login(&client, &cookie_string) {
                Ok(true) => {
                    info!("Using verified Bilibili login from {:?}", browser);
                    return Ok(cookie_string);
                }
                Ok(false) => info!("Skipping {:?}: Bilibili session is logged out", browser),
                Err(reason) => {
                    validation_failed = true;
                    warn!(
                        "Could not verify {:?} login: {}; trying next browser",
                        browser, reason
                    );
                }
            }
        }
    }
    if validation_failed {
        Err("No verified Bilibili session selected: login checks failed; check network/API availability and retry".into())
    } else {
        Err("No logged-in Bilibili session found; log into bilibili.com in your browser".into())
    }
}

fn validate_browser_login(
    client: &reqwest::blocking::Client,
    cookies: &str,
) -> Result<bool, String> {
    let mut header = reqwest::header::HeaderValue::from_str(cookies)
        .map_err(|_| "invalid cookie header".to_string())?;
    header.set_sensitive(true);
    let response = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header(reqwest::header::COOKIE, header)
        .header(reqwest::header::REFERER, "https://www.bilibili.com/")
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                "login check timed out (not a logout)".to_string()
            } else {
                "login request failed (network/TLS error, not a logout)".to_string()
            }
        })?;
    if !response.status().is_success() {
        return Err(format!("login API returned HTTP {}", response.status()));
    }
    let body: serde_json::Value = response
        .json()
        .map_err(|_| "login API returned invalid JSON".to_string())?;
    parse_login_status(&body)
}

fn parse_login_status(body: &serde_json::Value) -> Result<bool, String> {
    match body.get("code").and_then(serde_json::Value::as_i64) {
        Some(-101) => Ok(false),
        Some(0) => body
            .get("data")
            .and_then(|data| data.get("isLogin"))
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| "login API response is missing a boolean isLogin".to_string()),
        Some(code) => Err(format!(
            "login API returned code {} (not a confirmed logout)",
            code
        )),
        None => Err("login API response is missing a numeric code".to_string()),
    }
}

fn select_browser_cookies(cookies: Vec<Cookie>) -> Option<String> {
    let now = Utc::now();
    let mut valid: Vec<_> = cookies
        .into_iter()
        .filter(|cookie| {
            !cookie.value.is_empty() && cookie.expires.is_none_or(|expiry| expiry > now)
        })
        .collect();
    // Prefer shared Bilibili cookies over a subdomain's same-named cookie.
    valid.sort_by(|a, b| {
        let is_root = |domain: &str| domain.trim_start_matches('.') == "bilibili.com";
        a.name
            .cmp(&b.name)
            .then_with(|| is_root(&b.domain).cmp(&is_root(&a.domain)))
            .then_with(|| b.expires.cmp(&a.expires))
    });
    valid.dedup_by(|a, b| a.name == b.name);
    if !valid.iter().any(|cookie| cookie.name == "SESSDATA") {
        return None;
    }
    Some(
        valid
            .iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Get all bilibili cookies from browsers for debugging
pub fn get_all_bilibili_cookies() -> HashMap<String, String> {
    let mut all_cookies = HashMap::new();
    let browsers = Browser::get_all_supported();

    for browser in browsers {
        if let Ok(cookies) = read_cookies_from_browser(&browser, Some("bilibili.com")) {
            for cookie in cookies {
                // Only include non-expired cookies
                if let Some(expires) = cookie.expires {
                    if Utc::now() > expires {
                        continue;
                    }
                }

                // Use the most recent cookie if duplicates exist
                let key = format!("{}_{}", cookie.name, cookie.domain);
                all_cookies.insert(key, cookie.value);
            }
        }
    }

    all_cookies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_validation_distinguishes_logout_from_api_failure() {
        assert_eq!(
            parse_login_status(&serde_json::json!({"code": 0, "data": {"isLogin": true}})),
            Ok(true)
        );
        assert_eq!(
            parse_login_status(&serde_json::json!({"code": 0, "data": {"isLogin": false}})),
            Ok(false)
        );
        assert_eq!(
            parse_login_status(&serde_json::json!({"code": -101})),
            Ok(false)
        );
        for body in [
            serde_json::json!({"code": -412}),
            serde_json::json!({"code": 0, "data": {}}),
            serde_json::json!({"code": 0, "data": {"isLogin": "true"}}),
            serde_json::json!({"code": "0"}),
            serde_json::json!({}),
        ] {
            assert!(parse_login_status(&body).is_err());
        }
    }

    #[test]
    fn cookie_selection_requires_session_and_keeps_matching_csrf() {
        let cookie = |name: &str, value: &str, domain: &str| Cookie {
            name: name.into(),
            value: value.into(),
            domain: domain.into(),
            path: "/".into(),
            expires: None,
            secure: true,
            http_only: true,
        };
        assert!(
            select_browser_cookies(vec![cookie("bili_jct", "csrf", ".bilibili.com")]).is_none()
        );
        assert!(select_browser_cookies(vec![cookie("SESSDATA", "", ".bilibili.com")]).is_none());
        let selected = select_browser_cookies(vec![
            cookie("SESSDATA", "chrome-session", ".bilibili.com"),
            cookie("bili_jct", "chrome-csrf", ".bilibili.com"),
            cookie("SESSDATA", "subdomain-session", "live.bilibili.com"),
        ])
        .unwrap();
        assert!(selected.contains("SESSDATA=chrome-session"));
        assert!(selected.contains("bili_jct=chrome-csrf"));
        assert!(!selected.contains("subdomain-session"));
    }

    #[test]
    fn test_browser_path_detection() {
        for browser in Browser::get_all_supported() {
            let path = browser.get_cookie_db_path();
            println!("{:?} cookie path: {:?}", browser, path);
        }
    }

    #[test]
    fn test_find_sessdata() {
        // This test will only work if you have bilibili cookies in your browser
        if let Some(sessdata) = find_bilibili_cookies_as_string() {
            println!("Found a verified Bilibili session");
            assert!(!sessdata.is_empty());
        } else {
            println!("No SESSDATA found - this is normal if you're not logged into bilibili");
        }
    }
}
