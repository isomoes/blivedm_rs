// src/client/models.rs
//! Data models for Bilibili live danmaku WebSocket client

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DanmuServer {
    pub host: String,
    pub port: i32,
    pub wss_port: i32,
    pub ws_port: i32,
}

impl Default for DanmuServer {
    fn default() -> Self {
        Self {
            host: String::from("broadcastlv.chat.bilibili.com"),
            port: 2243,
            wss_port: 443,
            ws_port: 2244,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MsgHead {
    pub pack_len: u32,
    pub raw_header_size: u16,
    pub ver: u16,
    pub operation: u32,
    pub seq_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthMessage {
    pub uid: u64,
    pub roomid: u64,
    pub protover: i32,
    pub platform: String,
    pub type_: i32,
    pub key: String,
}

impl AuthMessage {
    pub fn from(map: &HashMap<String, String>) -> AuthMessage {
        AuthMessage {
            uid: map.get("uid").unwrap().parse::<u64>().unwrap(),
            roomid: map.get("room_id").unwrap().parse::<u64>().unwrap(),
            protover: 3,
            platform: "web".to_string(),
            type_: 2,
            key: map.get("token").unwrap().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BiliMessage {
    Danmu {
        user: String,
        text: String,
    },
    Gift {
        user: String,
        gift: String,
        num: String,
    },
    /// Online rank count message (ONLINE_RANK_COUNT)
    OnlineRankCount {
        /// Number of high-energy users in the live room
        count: u64,
        /// Number of online users in the live room
        online_count: u64,
    },
    // Add more variants as needed
    Raw(serde_json::Value),
    #[deprecated(note = "Use Raw variant instead")]
    Unsupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_message_from_map() {
        let mut map = std::collections::HashMap::new();
        map.insert("uid".to_string(), "12345".to_string());
        map.insert("room_id".to_string(), "67890".to_string());
        map.insert("token".to_string(), "test_token".to_string());
        let auth = AuthMessage::from(&map);
        assert_eq!(auth.uid, 12345);
        assert_eq!(auth.roomid, 67890);
        assert_eq!(auth.key, "test_token");
    }
}
