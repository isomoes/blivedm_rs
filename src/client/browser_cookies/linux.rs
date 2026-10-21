//! Decode Linux Chromium cookies using the same desktop keyring as the browser.
//! `secret-tool` (libsecret) is required for v11 cookies; secrets never enter logs.

use super::Browser;
use openssl::{hash::MessageDigest, pkcs5, sha::sha256, symm};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn derive_key(password: &[u8]) -> Result<Vec<u8>, String> {
    let mut key = vec![0; 16];
    pkcs5::pbkdf2_hmac(password, b"saltysalt", 1, MessageDigest::sha1(), &mut key)
        .map_err(|_| "Could not derive Chromium cookie key".to_string())?;
    Ok(key)
}

pub(super) fn keyring_key(browser: &Browser) -> Result<Vec<u8>, String> {
    let application = match browser {
        Browser::Chrome => "chrome",
        Browser::Chromium => "chromium",
        Browser::Edge => "microsoft-edge",
        Browser::Opera => "opera",
        Browser::Firefox => return Err("Firefox does not use Chromium encryption".into()),
    };
    let mut child = Command::new("secret-tool")
        .args(["lookup", "application", application])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            "Install secret-tool (libsecret) to read encrypted Chromium cookies".to_string()
        })?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(50));
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(if result.is_err() {
                    "Could not query desktop keyring"
                } else {
                    "Desktop keyring lookup timed out; unlock your keyring and retry"
                }
                .into());
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "Could not read desktop keyring response".to_string())?;
    let password = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    if !output.status.success() || password.is_empty() {
        return Err("Chromium key unavailable; unlock your desktop keyring and retry".into());
    }
    derive_key(password)
}

pub(super) fn decrypt(
    encrypted: &[u8],
    domain: &str,
    version: u32,
    key: Option<&[u8]>,
) -> Result<String, String> {
    let key = if encrypted.starts_with(b"v10") {
        derive_key(b"peanuts")?
    } else if encrypted.starts_with(b"v11") {
        key.ok_or("Chromium desktop keyring key unavailable")?
            .to_vec()
    } else {
        return Err("Unsupported Chromium cookie encryption version".into());
    };
    let plaintext = symm::decrypt(
        symm::Cipher::aes_128_cbc(),
        &key,
        Some(&[b' '; 16]),
        &encrypted[3..],
    )
    .map_err(|_| "Could not decrypt Chromium cookie".to_string())?;
    // Database version 24 added a SHA-256 host binding before each cookie value.
    let value = if version >= 24 {
        if !plaintext.starts_with(&sha256(domain.as_bytes())) {
            return Err("Chromium cookie host binding mismatch".into());
        }
        &plaintext[32..]
    } else {
        &plaintext[..]
    };
    String::from_utf8(value.to_vec()).map_err(|_| "Invalid Chromium cookie encoding".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypted(prefix: &[u8], key: &[u8], plaintext: &[u8]) -> Vec<u8> {
        let mut bytes = prefix.to_vec();
        bytes.extend(
            symm::encrypt(
                symm::Cipher::aes_128_cbc(),
                key,
                Some(&[b' '; 16]),
                plaintext,
            )
            .unwrap(),
        );
        bytes
    }

    #[test]
    fn decrypts_legacy_and_host_bound_cookies() {
        let key = derive_key(b"test-keyring-password").unwrap();
        let old = encrypted(b"v11", &key, b"test-session");
        assert_eq!(
            decrypt(&old, ".bilibili.com", 23, Some(&key)).unwrap(),
            "test-session"
        );
        let mut bound = sha256(b".bilibili.com").to_vec();
        bound.extend(b"test-session");
        let new = encrypted(b"v11", &key, &bound);
        assert_eq!(
            decrypt(&new, ".bilibili.com", 24, Some(&key)).unwrap(),
            "test-session"
        );
        assert!(decrypt(&new, ".example.com", 24, Some(&key)).is_err());
        assert!(decrypt(&new, ".bilibili.com", 24, None).is_err());
        assert!(decrypt(b"v11bad", ".bilibili.com", 24, Some(&key)).is_err());
        assert!(decrypt(b"v20", ".bilibili.com", 24, Some(&key)).is_err());
        let legacy = encrypted(b"v10", &derive_key(b"peanuts").unwrap(), b"legacy");
        assert_eq!(
            decrypt(&legacy, ".bilibili.com", 23, None).unwrap(),
            "legacy"
        );
    }
}
