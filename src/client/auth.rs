// src/client/auth.rs
//! Authentication helpers for Bilibili live danmaku WebSocket client

use md5;
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

// Add browser cookie support
#[cfg(feature = "browser_cookies")]
use crate::browser_cookies;

/// Get Bilibili cookies from browser (preferred, newest), then fallback to provided cookie string
pub fn get_cookies_or_browser(provided_cookie: Option<&str>) -> Option<String> {
    #[cfg(feature = "browser_cookies")]
    {
        // First try browser cookies as they are the newest
        log::info!("Searching for Bilibili cookies in browser (newest)...");
        if let Some(browser_cookie) = browser_cookies::find_bilibili_cookies_as_string() {
            log::info!("Found Bilibili cookies in browser (using newest)");
            return Some(browser_cookie);
        }

        log::info!("No Bilibili cookies found in browser, checking provided cookie...");
    }

    #[cfg(not(feature = "browser_cookies"))]
    {
        log::debug!("Browser cookie feature not enabled. Skip to find cookies in browser...");
    }
    
    if let Some(cookie) = provided_cookie {
        if !cookie.is_empty() && cookie != "dummy_sessdata" && cookie.len() > 20 {
            log::info!("Using provided cookie as fallback");
            return Some(cookie.to_string());
        }
    }

    log::warn!("No valid Bilibili cookies found in browser or provided input");
    None
}

pub fn init_uid(headers: HeaderMap) -> (StatusCode, String) {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .unwrap();

    let mut request_headers = headers;
    request_headers.insert("user-agent", USER_AGENT.parse().unwrap());

    let response = client.get(UID_INIT_URL).headers(request_headers).send();
    log::debug!("init uid response: {:?}", response);
    let stat: StatusCode;
    let body: String;
    match response {
        Ok(resp) => {
            stat = resp.status();
            body = resp.text().unwrap();
            log::info!("init uid response: {:?}", body);
        }
        Err(_) => {
            panic!("init uid failed");
        }
    }
    (stat, body)
}

/// Initializes the buvid by sending a request and extracting the 'buvid3' cookie.
///
/// Note: This function is not used for document creation.
///
/// # Panics
///
/// Panics if the request fails.
pub fn init_buvid(headers: HeaderMap) -> (StatusCode, String) {
    // Not used for document creation.
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .unwrap();

    let mut request_headers = headers;
    request_headers.insert("user-agent", USER_AGENT.parse().unwrap());

    let response = client.get(BUVID_INIT_URL).headers(request_headers).send();
    let stat: StatusCode;
    let mut buvid: String = "".to_string();
    match response {
        Ok(resp) => {
            stat = resp.status();
            let cookies = resp.cookies();
            for i in cookies {
                log::debug!("init buvid response cookie : {:?}", i);
                if "buvid3".eq(i.name()) {
                    buvid = i.value().to_string();
                    log::info!("init buvid response: {:?}", buvid);
                }
            }
        }
        Err(_) => {
            panic!("init buvid failed");
        }
    }
    (stat, buvid)
}

/// Initializes the room by sending a request with the given room ID.
///
/// Note: This function should NOT be used for document creation.
///
/// # Panics
///
/// Panics if the request fails.
pub fn init_room(headers: HeaderMap, temp_room_id: &str) -> (StatusCode, String) {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .unwrap();

    let mut request_headers = headers;
    request_headers.insert("user-agent", USER_AGENT.parse().unwrap());

    let url = format!("{}?room_id={}", ROOM_INIT_URL, temp_room_id);
    let response = client.get(url).headers(request_headers).send();
    let stat: StatusCode;
    let body: String;
    match response {
        Ok(resp) => {
            stat = resp.status();
            body = resp.text().unwrap();
            log::info!("init room response: {:?}", body);
        }
        Err(_) => {
            panic!("init buvid failed");
        }
    }
    (stat, body)
}

pub fn init_host_server(headers: HeaderMap, room_id: u64) -> (StatusCode, String) {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .unwrap();

    let mut request_headers = headers.clone();
    request_headers.insert("user-agent", USER_AGENT.parse().unwrap());

    // Get WBI keys for signing
    let wbi_keys = match get_wbi_keys(request_headers.clone()) {
        Ok(keys) => keys,
        Err(e) => {
            log::error!("Failed to get WBI keys: {:?}", e);
            panic!("Failed to get WBI keys");
        }
    };

    // Prepare parameters for signing
    let params = vec![
        ("id", room_id.to_string()),
        ("type", "0".to_string()),
        ("web_location", "444.8".to_string()),
    ];

    // Generate signed query string
    let signed_query = encode_wbi(params, wbi_keys);

    // Construct final URL
    let url = format!("{}?{}", DANMAKU_SERVER_CONF_URL, signed_query);

    // debug log the total request
    let response = client.get(url).headers(request_headers).send();
    log::debug!("init host server response: {:?}", response);
    let stat: StatusCode;
    let body: String;
    match response {
        Ok(resp) => {
            stat = resp.status();
            body = resp.text().unwrap();
            log::info!("init host server response body: {:?}", body);
        }
        Err(_) => {
            panic!("init host server failed");
        }
    }
    (stat, body)
}

// WBI signing constants and functions
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

#[derive(Deserialize)]
struct WbiImg {
    img_url: String,
    sub_url: String,
}

#[derive(Deserialize)]
struct Data {
    wbi_img: WbiImg,
}

#[derive(Deserialize)]
struct ResWbi {
    data: Data,
}

// 对 imgKey 和 subKey 进行字符顺序打乱编码
fn get_mixin_key(orig: &[u8]) -> String {
    MIXIN_KEY_ENC_TAB
        .iter()
        .take(32)
        .map(|&i| orig[i] as char)
        .collect::<String>()
}

fn get_url_encoded(s: &str) -> String {
    s.chars()
        .filter_map(|c| match c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            true => Some(c.to_string()),
            false => {
                // 过滤 value 中的 "!'()*" 字符
                if "!'()*".contains(c) {
                    return None;
                }
                let encoded = c
                    .encode_utf8(&mut [0; 4])
                    .bytes()
                    .fold("".to_string(), |acc, b| acc + &format!("%{:02X}", b));
                Some(encoded)
            }
        })
        .collect::<String>()
}

// 为请求参数进行 wbi 签名
fn encode_wbi(params: Vec<(&str, String)>, (img_key, sub_key): (String, String)) -> String {
    let cur_time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };
    _encode_wbi(params, (img_key, sub_key), cur_time)
}

fn _encode_wbi(
    mut params: Vec<(&str, String)>,
    (img_key, sub_key): (String, String),
    timestamp: u64,
) -> String {
    let mixin_key = get_mixin_key((img_key + &sub_key).as_bytes());
    // 添加当前时间戳
    params.push(("wts", timestamp.to_string()));
    // 重新排序
    params.sort_by(|a, b| a.0.cmp(b.0));
    // 拼接参数
    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", get_url_encoded(k), get_url_encoded(v)))
        .collect::<Vec<_>>()
        .join("&");
    // 计算签名
    let web_sign = format!("{:x}", md5::compute(query.clone() + &mixin_key));
    // 返回最终的 query
    query + &format!("&w_rid={}", web_sign)
}

fn get_wbi_keys(headers: HeaderMap) -> Result<(String, String), reqwest::Error> {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .build()
        .unwrap();

    let mut request_headers = headers;
    request_headers.insert("user-agent", USER_AGENT.parse().unwrap());

    let response = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .headers(request_headers)
        .send()?;

    let res_wbi: ResWbi = response.json()?;
    Ok((
        take_filename(res_wbi.data.wbi_img.img_url).unwrap(),
        take_filename(res_wbi.data.wbi_img.sub_url).unwrap(),
    ))
}

fn take_filename(url: String) -> Option<String> {
    url.rsplit_once('/')
        .and_then(|(_, s)| s.rsplit_once('.'))
        .map(|(s, _)| s.to_string())
}

pub const UID_INIT_URL: &str = "https://api.bilibili.com/x/web-interface/nav";
pub const BUVID_INIT_URL: &str = "https://data.bilibili.com/v/";
pub const ROOM_INIT_URL: &str =
    "https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom";
pub const DANMAKU_SERVER_CONF_URL: &str =
    "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo";
pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uid_url_constant() {
        assert!(UID_INIT_URL.contains("bilibili.com"));
    }

    #[test]
    fn test_take_filename() {
        assert_eq!(
            take_filename(
                "https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png".to_string()
            ),
            Some("7cd084941338484aae1ad9425b84077c".to_string())
        );

        assert_eq!(
            take_filename(
                "https://i0.hdslb.com/bfs/wbi/4932caff0ff746eab6f01bf08b70ac45.png".to_string()
            ),
            Some("4932caff0ff746eab6f01bf08b70ac45".to_string())
        );

        // Test edge case with no extension
        assert_eq!(
            take_filename("https://example.com/path/file".to_string()),
            None
        );
    }

    #[test]
    fn test_encode_wbi_with_known_values() {
        let params = vec![
            ("foo", String::from("114")),
            ("bar", String::from("514")),
            ("zab", String::from("1919810")),
        ];

        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1702204169,
        );

        assert_eq!(
            result,
            "bar=514&foo=114&wts=1702204169&zab=1919810&w_rid=8f6f2b5b3d485fe1886cec6a0be8c5d4"
        );
    }

    #[test]
    fn test_encode_wbi_bilibili_danmu_params() {
        // Test with the actual Bilibili danmu parameters from the example
        let params = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        // Using the timestamp from the example URL (1748308267)
        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1748308267,
        );

        // The result should contain the correct parameters and w_rid
        assert!(result.contains("id=24779526"));
        assert!(result.contains("type=0"));
        assert!(result.contains("web_location=444.8"));
        assert!(result.contains("wts=1748308267"));
        assert!(result.contains("w_rid="));

        // Check the parameter order (should be alphabetical)
        let expected_order = "id=24779526&type=0&web_location=444.8&wts=1748308267&w_rid=";
        assert!(result.starts_with(expected_order));
    }

    #[test]
    fn test_wbi_signature_consistency() {
        // Test that the same parameters always generate the same signature
        let params1 = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        let params2 = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        let keys = (
            "7cd084941338484aae1ad9425b84077c".to_string(),
            "4932caff0ff746eab6f01bf08b70ac45".to_string(),
        );

        let timestamp = 1748308267;

        let result1 = _encode_wbi(params1, keys.clone(), timestamp);
        let result2 = _encode_wbi(params2, keys, timestamp);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_wbi_parameter_sorting() {
        // Test that parameters are properly sorted alphabetically
        let params = vec![
            ("z_param", String::from("last")),
            ("a_param", String::from("first")),
            ("m_param", String::from("middle")),
        ];

        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1748308267,
        );

        // Check that parameters appear in alphabetical order
        let parts: Vec<&str> = result.split('&').collect();
        assert!(parts[0].starts_with("a_param="));
        assert!(parts[1].starts_with("m_param="));
        assert!(parts[2].starts_with("wts="));
        assert!(parts[3].starts_with("z_param="));
        assert!(parts[4].starts_with("w_rid="));
    }

    #[test]
    fn test_correct_bilibili_url_signature() {
        // Test the exact URL from the working example:
        // "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id=24779526&type=0&web_location=444.8&wts=1748308267&w_rid=884cf361b8ad4e239b4a9dbbb7134679"
        // "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id=24779526&type=0&web_location=444.8&w_rid=d1e619744b4977f88ed67524a1f567cc&wts=1751072897"

        let params = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1751072897,
        );

        // Expected complete query string from working URL
        let expected = "id=24779526&type=0&web_location=444.8&wts=1751072897&w_rid=d1e619744b4977f88ed67524a1f567cc";
        assert_eq!(result, expected);

        // Extract and verify the w_rid specifically
        let w_rid = result.split("w_rid=").nth(1).unwrap();
        assert_eq!(w_rid, "d1e619744b4977f88ed67524a1f567cc");
    }

    #[test]
    fn test_second_bilibili_url_signature() {
        // Test the second URL example:
        // "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id=24779526&type=0&web_location=444.8&w_rid=fa20533eb27334ba6f2ec7263721319a&wts=1748311635"

        let params = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1748311635,
        );

        // Expected complete query string from working URL
        let expected = "id=24779526&type=0&web_location=444.8&wts=1748311635&w_rid=fa20533eb27334ba6f2ec7263721319a";
        assert_eq!(result, expected);

        // Extract and verify the w_rid specifically
        let w_rid = result.split("w_rid=").nth(1).unwrap();
        assert_eq!(w_rid, "fa20533eb27334ba6f2ec7263721319a");
    }

    #[test]
    fn test_third_bilibili_url_signature() {
        // Test the third URL example from README:
        // "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id=24779526&type=0&web_location=444.8&wts=1748312554&w_rid=30f250e8abd9effea1bcb88aab416507"

        let params = vec![
            ("id", String::from("24779526")),
            ("type", String::from("0")),
            ("web_location", String::from("444.8")),
        ];

        let result = _encode_wbi(
            params,
            (
                "7cd084941338484aae1ad9425b84077c".to_string(),
                "4932caff0ff746eab6f01bf08b70ac45".to_string(),
            ),
            1748312554,
        );

        // Expected complete query string from working URL
        let expected = "id=24779526&type=0&web_location=444.8&wts=1748312554&w_rid=30f250e8abd9effea1bcb88aab416507";
        assert_eq!(result, expected);

        // Extract and verify the w_rid specifically
        let w_rid = result.split("w_rid=").nth(1).unwrap();
        assert_eq!(w_rid, "30f250e8abd9effea1bcb88aab416507");
    }
}
