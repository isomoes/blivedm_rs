use crate::client::models::BiliMessage;
use crate::client::scheduler::{EventContext, EventHandler};
use log::{debug, error, info};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Configuration for keyword-response triggers
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// Keywords that trigger this response
    pub keywords: Vec<String>,
    /// Response message to send
    pub response: String,
}

/// Configuration for the auto reply plugin
#[derive(Debug, Clone)]
pub struct AutoReplyConfig {
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Minimum cooldown between replies in seconds
    pub cooldown_seconds: u64,
    /// List of trigger configurations
    pub triggers: Vec<TriggerConfig>,
}

impl Default for AutoReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cooldown_seconds: 5,
            triggers: vec![
                TriggerConfig {
                    keywords: vec!["你好".to_string(), "hello".to_string()],
                    response: "欢迎来到直播间！".to_string(),
                },
                TriggerConfig {
                    keywords: vec!["谢谢".to_string(), "thanks".to_string()],
                    response: "不客气～".to_string(),
                },
            ],
        }
    }
}

/// Parameters for sending a danmaku message to Bilibili API
#[derive(Serialize, Debug)]
struct SendDanmakuRequest {
    csrf: String,
    roomid: u64,
    msg: String,
    rnd: u64,
    fontsize: u32,
    color: u32,
    mode: u32,
    bubble: u32,
    room_type: u32,
    jumpfrom: u32,
    reply_mid: u32,
    reply_attr: u32,
    reply_uname: String,
    replay_dmid: String,
    statistics: String,
    csrf_token: String,
}

/// Extract CSRF token from cookies string
pub fn extract_csrf_token(cookies: &str) -> Option<String> {
    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if cookie.starts_with("bili_jct=") {
            return (!cookie[9..].is_empty()).then(|| cookie[9..].to_string());
        }
    }
    None
}

/// Send a danmaku message to the Bilibili live room
///
/// # Arguments
/// * `message` - The text message to send
/// * `context` - Event context containing cookies and room_id
///
/// # Returns
/// Returns Ok(()) on success, or an error if the request fails
pub async fn send_danmaku_message(
    message: &str,
    context: &EventContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    send_danmaku_with_client(&http_client, message, context).await
}

// Manual sends and auto-replies must validate the same HTTP and API responses.
async fn send_danmaku_with_client(
    http_client: &reqwest::Client,
    message: &str,
    context: &EventContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let cookies = match &context.cookies {
        Some(cookies) => cookies,
        None => {
            return Err("No cookies available for sending danmaku".into());
        }
    };

    let csrf_token = match extract_csrf_token(cookies) {
        Some(token) => token,
        None => {
            return Err("Could not extract CSRF token from cookies".into());
        }
    };

    // Current timestamp
    let rnd = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = SendDanmakuRequest {
        csrf: csrf_token.clone(),
        roomid: context.room_id,
        msg: message.to_string(),
        rnd,
        fontsize: 25,
        color: 16777215, // White color
        mode: 1,         // Scroll mode
        bubble: 0,
        room_type: 0,
        jumpfrom: 0,
        reply_mid: 0,
        reply_attr: 0,
        reply_uname: String::new(),
        replay_dmid: String::new(),
        statistics: r#"{"appId":100,"platform":5}"#.to_string(),
        csrf_token,
    };

    // Set up headers
    let mut headers = HeaderMap::new();
    headers.insert("Cookie", HeaderValue::from_str(cookies)?);
    headers.insert(
        "User-Agent",
        HeaderValue::from_static(
            "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0",
        ),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_str(&format!("https://live.bilibili.com/{}", context.room_id))?,
    );

    debug!("Sending danmaku: {}", message);

    let response = http_client
        .post("https://api.live.bilibili.com/msg/send")
        .headers(headers)
        .form(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("Failed to send danmaku: HTTP {}", status).into());
    }
    let body: serde_json::Value = response.json().await?;
    validate_send_response(&body)?;
    info!("Successfully sent danmaku: {}", message);
    Ok(())
}

fn validate_send_response(body: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let code = body
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .ok_or("Invalid Bilibili send response: missing numeric code")?;
    let message = ["message", "msg"]
        .iter()
        .filter_map(|key| body.get(key).and_then(serde_json::Value::as_str))
        .find(|message| !message.is_empty());
    // Some moderation/rate-limit responses have code 0 but a rejection in msg.
    let rejection = body
        .get("msg")
        .and_then(serde_json::Value::as_str)
        .filter(|message| !message.is_empty());
    if code != 0 || rejection.is_some() {
        return Err(format!(
            "Bilibili rejected danmaku (code {}): {}",
            code,
            rejection.or(message).unwrap_or("No reason provided")
        )
        .into());
    }
    Ok(())
}

/// Auto reply handler that monitors danmaku for keywords and sends responses
pub struct AutoReplyHandler {
    config: AutoReplyConfig,
    last_reply: Arc<Mutex<Option<Instant>>>,
    http_client: reqwest::Client,
    runtime: Arc<Runtime>,
}

impl AutoReplyHandler {
    /// Create a new auto reply handler with the given configuration
    pub fn new(config: AutoReplyConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));

        Self {
            config,
            last_reply: Arc::new(Mutex::new(None)),
            http_client,
            runtime,
        }
    }

    /// Check if any keyword matches the message text
    fn find_matching_trigger(&self, text: &str) -> Option<&TriggerConfig> {
        let text_lower = text.to_lowercase();

        for trigger in &self.config.triggers {
            for keyword in &trigger.keywords {
                if text_lower.contains(&keyword.to_lowercase()) {
                    return Some(trigger);
                }
            }
        }

        None
    }

    /// Get the response from the trigger
    fn select_response(&self, trigger: &TriggerConfig) -> Option<String> {
        if trigger.response.is_empty() {
            return None;
        }
        Some(trigger.response.clone())
    }

    /// Check if enough time has passed since the last reply
    fn check_cooldown(&self) -> bool {
        let last_reply = self.last_reply.lock().unwrap();

        match *last_reply {
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                elapsed >= Duration::from_secs(self.config.cooldown_seconds)
            }
            None => true,
        }
    }

    /// Update the last reply timestamp
    fn update_last_reply(&self) {
        let mut last_reply = self.last_reply.lock().unwrap();
        *last_reply = Some(Instant::now());
    }

    /// Extract CSRF token from cookies
    #[cfg(test)]
    fn extract_csrf_token(&self, cookies: &str) -> Option<String> {
        extract_csrf_token(cookies)
    }

    /// Send a danmaku message to the Bilibili API
    async fn send_danmaku(
        &self,
        message: &str,
        context: &EventContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        send_danmaku_with_client(&self.http_client, message, context).await
    }
}

impl EventHandler for AutoReplyHandler {
    fn handle(&self, msg: &BiliMessage, context: &EventContext) {
        if !self.config.enabled {
            return;
        }

        // Only process danmaku messages
        if let BiliMessage::Danmu { user: _, text } = msg {
            // Check for keyword match
            if let Some(trigger) = self.find_matching_trigger(text) {
                // Check cooldown
                if !self.check_cooldown() {
                    debug!("Auto reply on cooldown, skipping");
                    return;
                }

                // Select response
                if let Some(response) = self.select_response(trigger) {
                    debug!(
                        "Auto reply triggered by '{}', responding with '{}'",
                        text, response
                    );

                    // Update cooldown
                    self.update_last_reply();

                    // Send the reply asynchronously
                    let runtime = Arc::clone(&self.runtime);
                    let response_msg = response.clone();
                    let context_clone = context.clone();
                    let handler = self.clone();

                    runtime.spawn(async move {
                        if let Err(e) = handler.send_danmaku(&response_msg, &context_clone).await {
                            error!("Failed to send auto reply: {}", e);
                        }
                    });
                }
            }
        }
    }
}

// Implement Clone for AutoReplyHandler
impl Clone for AutoReplyHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            last_reply: Arc::clone(&self.last_reply),
            http_client: self.http_client.clone(),
            runtime: Arc::clone(&self.runtime),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::models::BiliMessage;
    use crate::client::scheduler::{EventContext, EventHandler};

    #[test]
    fn test_send_response_validation() {
        for body in [
            serde_json::json!({"code": 0, "message": "", "msg": ""}),
            serde_json::json!({"code": 0, "message": "OK"}),
        ] {
            assert!(validate_send_response(&body).is_ok());
        }
        for body in [
            serde_json::json!({"code": -101, "message": "Not logged in"}),
            serde_json::json!({"code": -111, "msg": "CSRF failed"}),
            serde_json::json!({"code": 0, "msg": "Message blocked"}),
            serde_json::json!({"message": "Missing code"}),
            serde_json::json!({"code": "0"}),
        ] {
            assert!(validate_send_response(&body).is_err());
        }
    }

    #[test]
    fn test_keyword_matching() {
        let config = AutoReplyConfig::default();
        let handler = AutoReplyHandler::new(config);

        // Test keyword matching
        assert!(handler.find_matching_trigger("你好世界").is_some());
        assert!(handler.find_matching_trigger("Hello world").is_some());
        assert!(handler.find_matching_trigger("谢谢大家").is_some());
        assert!(handler.find_matching_trigger("Thanks everyone").is_some());
        assert!(handler.find_matching_trigger("random text").is_none());
    }

    #[test]
    fn test_response_selection() {
        let config = AutoReplyConfig::default();
        let handler = AutoReplyHandler::new(config);

        let trigger = &handler.config.triggers[0];
        let response = handler.select_response(trigger);
        assert!(response.is_some());
        assert_eq!(response.unwrap(), trigger.response);
    }

    #[test]
    fn test_cooldown() {
        let config = AutoReplyConfig {
            enabled: true,
            cooldown_seconds: 1,
            triggers: vec![],
        };
        let handler = AutoReplyHandler::new(config);

        // Initial check should pass
        assert!(handler.check_cooldown());

        // Update timestamp
        handler.update_last_reply();

        // Should be on cooldown now
        assert!(!handler.check_cooldown());

        // Wait for cooldown
        std::thread::sleep(Duration::from_secs(2));

        // Should be off cooldown now
        assert!(handler.check_cooldown());
    }

    #[test]
    fn test_csrf_extraction() {
        let config = AutoReplyConfig::default();
        let handler = AutoReplyHandler::new(config);

        let cookies = "SESSDATA=abc123; bili_jct=csrf_token_here; other=value";
        let csrf = handler.extract_csrf_token(cookies);
        assert_eq!(csrf, Some("csrf_token_here".to_string()));

        let cookies_no_csrf = "SESSDATA=abc123; other=value";
        let csrf = handler.extract_csrf_token(cookies_no_csrf);
        assert_eq!(csrf, None);
    }

    #[test]
    fn test_event_handler() {
        let config = AutoReplyConfig {
            enabled: true,
            cooldown_seconds: 0, // No cooldown for testing
            triggers: vec![TriggerConfig {
                keywords: vec!["test".to_string()],
                response: "test response".to_string(),
            }],
        };
        let handler = AutoReplyHandler::new(config);

        let context = EventContext {
            cookies: Some("bili_jct=test_csrf; SESSDATA=test".to_string()),
            room_id: 12345,
        };

        let msg = BiliMessage::Danmu {
            user: "test_user".to_string(),
            text: "this is a test message".to_string(),
        };

        // This should trigger the auto reply (but won't actually send due to test environment)
        handler.handle(&msg, &context);
    }
}
