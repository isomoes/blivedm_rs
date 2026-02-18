// src/client/websocket.rs
//! WebSocket client for Bilibili live danmaku messages (refactored from bili_live_dm)

use native_tls::TlsStream;
use serde_json::Value;
use std::net::TcpStream;
use tungstenite::{Message, WebSocket, client};

use url::Url;

use futures_channel::mpsc::Sender;
use http::Response;
use std::collections::HashMap;

use crate::auth::*;
use crate::models::{AuthMessage, BiliMessage, DanmuServer, MsgHead};

pub struct BiliLiveClient {
    ws: WebSocket<TlsStream<TcpStream>>,
    auth_msg: String,
    ss: Sender<BiliMessage>,
}

impl BiliLiveClient {
    pub fn new(cookies: &str, room_id: &str, r: Sender<BiliMessage>) -> Self {
        let (v, auth) = init_server(cookies, room_id);
        let (ws, _res) = connect(v["host_list"].clone());
        BiliLiveClient {
            ws,
            auth_msg: serde_json::to_string(&auth).unwrap(),
            ss: r,
        }
    }

    /// Create a new client with automatic browser cookie detection
    /// If cookies is None or empty, it will try to find cookies from browser
    pub fn new_auto(
        cookies: Option<&str>,
        room_id: &str,
        r: Sender<BiliMessage>,
    ) -> Result<Self, String> {
        let (v, auth) = init_server_auto(cookies, room_id)?;
        let (ws, _res) = connect(v["host_list"].clone());
        Ok(BiliLiveClient {
            ws,
            auth_msg: serde_json::to_string(&auth).unwrap(),
            ss: r,
        })
    }

    pub fn send_auth(&mut self) {
        let _ = self.ws.send(Message::Binary(make_packet(
            self.auth_msg.as_str(),
            Operation::AUTH,
        )));
    }

    pub fn send_heart_beat(&mut self) {
        let _ = self
            .ws
            .send(Message::Binary(make_packet("{}", Operation::HEARTBEAT)));
    }

    pub fn parse_ws_message(&mut self, resv: Vec<u8>) {
        let mut offset = 0;
        let header = &resv[0..16];
        let mut head_1 = get_msg_header(header);
        if head_1.operation == 5 || head_1.operation == 8 {
            loop {
                let body: &[u8] = &resv[offset + 16..offset + (head_1.pack_len as usize)];
                self.parse_business_message(head_1, body);
                offset += head_1.pack_len as usize;
                if offset >= resv.len() {
                    break;
                }
                let temp_head = &resv[offset..(offset + 16)];
                head_1 = get_msg_header(temp_head);
            }
        } else if head_1.operation == 3 {
            let mut body: [u8; 4] = [0, 0, 0, 0];
            body[0] = resv[16];
            body[1] = resv[17];
            body[2] = resv[18];
            body[3] = resv[19];
            let popularity = i32::from_be_bytes(body);
            log::info!("popularity:{}", popularity);
        } else {
            log::error!(
                "unknown message operation={:?}, header={:?}}}",
                head_1.operation,
                head_1
            )
        }
    }

    pub fn parse_business_message(&mut self, h: MsgHead, b: &[u8]) {
        if h.operation == 5 {
            if h.ver == 3 {
                let res: Vec<u8> = decompress(b).unwrap();
                self.parse_ws_message(res);
            } else if h.ver == 0 {
                let s = String::from_utf8(b.to_vec()).unwrap();
                let res_json: Value = serde_json::from_str(s.as_str()).unwrap();
                if let Some(msg) = handle(res_json) {
                    let _ = self.ss.try_send(msg);
                }
            } else {
                log::error!("Unknown compression format");
            }
        } else if h.operation == 8 {
            self.send_heart_beat();
        } else {
            log::error!("Unknown message format {}", h.operation);
        }
    }

    pub fn receive(&mut self) -> Result<(), String> {
        if self.ws.can_read() {
            let msg = self.ws.read();
            match msg {
                Ok(m) => {
                    let res = m.into_data();
                    if res.len() >= 16 {
                        self.parse_ws_message(res);
                    }
                    Ok(())
                }
                Err(e) => Err(format!("read msg error: {}", e)),
            }
        } else {
            Ok(())
        }
    }
}

pub fn gen_damu_list(list: &Value) -> Vec<DanmuServer> {
    let server_list = list.as_array().unwrap();
    let mut res: Vec<DanmuServer> = Vec::new();
    if server_list.len() == 0 {
        let d = DanmuServer::default();
        res.push(d);
    }
    for s in server_list {
        res.push(DanmuServer {
            host: s["host"].as_str().unwrap().to_string(),
            port: s["port"].as_u64().unwrap() as i32,
            wss_port: s["wss_port"].as_u64().unwrap() as i32,
            ws_port: s["ws_port"].as_u64().unwrap() as i32,
        });
    }
    res
}

fn find_server(vd: Vec<DanmuServer>) -> (String, String, String) {
    let (host, wss_port) = (vd.get(0).unwrap().host.clone(), vd.get(0).unwrap().wss_port);
    (
        host.clone(),
        format!("{}:{}", host.clone(), wss_port),
        format!("wss://{}:{}/sub", host, wss_port),
    )
}

pub fn init_server(cookies: &str, room_id: &str) -> (Value, AuthMessage) {
    let mut auth_map = HashMap::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::COOKIE,
        reqwest::header::HeaderValue::from_str(cookies).unwrap(),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(crate::auth::USER_AGENT),
    );
    log::debug!("headers: {:?}", headers);

    // Extract SESSDATA from cookies for authentication
    let sessdata = cookies
        .split(';')
        .find_map(|kv| {
            let mut parts = kv.trim().splitn(2, '=');
            let key = parts.next()?.trim();
            let value = parts.next()?.trim();
            if key == "SESSDATA" {
                Some(value.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "".to_string());

    if !sessdata.is_empty() {
        let (_, body1) = init_uid(headers.clone());
        let body1_v: Value = serde_json::from_str(body1.as_str()).unwrap();

        // Check if the authentication was successful
        if let Some(mid) = body1_v["data"]["mid"].as_i64() {
            auth_map.insert("uid".to_string(), mid.to_string());
            log::info!("Successfully authenticated with uid: {}", mid);
        } else {
            log::warn!("Authentication failed - SESSDATA may be invalid or expired");
            log::debug!("Auth response: {}", body1);
            auth_map.insert("uid".to_string(), "0".to_string());
        }
    } else {
        auth_map.insert("uid".to_string(), "0".to_string());
    }
    // here the live room id is easily obtained, so we not get it by url.
    auth_map.insert("room_id".to_string(), room_id.to_string());

    let room_id_num = room_id.parse::<u64>().expect("room_id must be a valid u64");
    let (_, body4) = init_host_server(headers.clone(), room_id_num);
    let body4_res: Value = serde_json::from_str(body4.as_str()).unwrap();
    let server_info = &body4_res["data"];
    let token = &body4_res["data"]["token"].as_str().unwrap();
    auth_map.insert("token".to_string(), token.to_string());

    let auth_msg = AuthMessage::from(&auth_map);
    (server_info.clone(), auth_msg)
}

pub fn connect(v: Value) -> (WebSocket<TlsStream<TcpStream>>, Response<Option<Vec<u8>>>) {
    let danmu_server = gen_damu_list(&v);
    let (host, url, ws_url) = find_server(danmu_server);
    let connector: native_tls::TlsConnector = native_tls::TlsConnector::new().unwrap();
    let stream: TcpStream = TcpStream::connect(url).unwrap();
    let stream: native_tls::TlsStream<TcpStream> =
        connector.connect(host.as_str(), stream).unwrap();
    let (socket, resp) =
        client(Url::parse(ws_url.as_str()).unwrap(), stream).expect("Can't connect");
    (socket, resp)
}

pub enum Operation {
    AUTH,
    HEARTBEAT,
}

pub fn make_packet(body: &str, ops: Operation) -> Vec<u8> {
    let json: Value = serde_json::from_str(body).unwrap();
    let temp = json.to_string();
    let body_content: &[u8] = temp.as_bytes();
    let pack_len: [u8; 4] = ((16 + body.len()) as u32).to_be_bytes();
    let raw_header_size: [u8; 2] = (16 as u16).to_be_bytes();
    let ver: [u8; 2] = (1 as u16).to_be_bytes();
    let operation: [u8; 4] = match ops {
        Operation::AUTH => (7 as u32).to_be_bytes(),
        Operation::HEARTBEAT => (2 as u32).to_be_bytes(),
    };
    let seq_id: [u8; 4] = (1 as u32).to_be_bytes();
    let mut res = pack_len.to_vec();
    res.append(&mut raw_header_size.to_vec());
    res.append(&mut ver.to_vec());
    res.append(&mut operation.to_vec());
    res.append(&mut seq_id.to_vec());
    res.append(&mut body_content.to_vec());
    res
}

pub fn get_msg_header(v_s: &[u8]) -> MsgHead {
    let mut pack_len: [u8; 4] = [0; 4];
    let mut raw_header_size: [u8; 2] = [0; 2];
    let mut ver: [u8; 2] = [0; 2];
    let mut operation: [u8; 4] = [0; 4];
    let mut seq_id: [u8; 4] = [0; 4];
    for (i, v) in v_s.iter().enumerate() {
        if i < 4 {
            pack_len[i] = *v;
            continue;
        }
        if i < 6 {
            raw_header_size[i - 4] = *v;
            continue;
        }
        if i < 8 {
            ver[i - 6] = *v;
            continue;
        }
        if i < 12 {
            operation[i - 8] = *v;
            continue;
        }
        if i < 16 {
            seq_id[i - 12] = *v;
            continue;
        }
    }
    MsgHead {
        pack_len: u32::from_be_bytes(pack_len),
        raw_header_size: u16::from_be_bytes(raw_header_size),
        ver: u16::from_be_bytes(ver),
        operation: u32::from_be_bytes(operation),
        seq_id: u32::from_be_bytes(seq_id),
    }
}

pub fn decompress(body: &[u8]) -> std::io::Result<Vec<u8>> {
    use brotlic::DecompressorReader;
    use std::io::Read;
    let mut decompressed_reader: DecompressorReader<&[u8]> = DecompressorReader::new(body);
    let mut decoded_input = Vec::new();
    let _ = decompressed_reader.read_to_end(&mut decoded_input)?;
    Ok(decoded_input)
}

/// here we detail [info format is online](https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/docs/live/message_stream.md)
/// .
pub fn handle(json: Value) -> Option<BiliMessage> {
    let category = json["cmd"].as_str().unwrap_or("");
    let data = json.get("data").unwrap_or(&json);
    match category {
        "DANMU_MSG" => Some(BiliMessage::Danmu {
            user: json["info"][2][1]
                .as_str()
                .unwrap_or("<unknown>")
                .to_string(),
            text: json["info"][1].as_str().unwrap_or("").to_string(),
        }),
        "SEND_GIFT" => Some(BiliMessage::Gift {
            user: data["uname"]
                .as_str()
                .or_else(|| data["sender_uinfo"]["base"]["name"].as_str())
                .unwrap_or("<unknown>")
                .to_string(),
            gift: data["giftName"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            num: data["num"].as_u64().unwrap_or(1).to_string(),
        }),
        "ONLINE_RANK_COUNT" => Some(BiliMessage::OnlineRankCount {
            count: json["data"]["count"].as_u64().unwrap_or(0),
            online_count: json["data"]["online_count"].as_u64().unwrap_or(0),
        }),
        // Add more cases for other types as needed
        _ => Some(BiliMessage::Raw(json)),
    }
}

/// Enhanced init_server that can automatically detect cookies from browser
pub fn init_server_auto(
    provided_cookies: Option<&str>,
    room_id: &str,
) -> Result<(Value, AuthMessage), String> {
    // Try to get cookies from provided value or browser cookies
    let cookies = get_cookies_or_browser(provided_cookies)
        .ok_or_else(|| "No cookies found in provided value or browser cookies. Please log into bilibili.com in your browser or provide cookies manually.".to_string())?;

    log::info!(
        "Using cookies for authentication: {}...",
        &cookies[..10.min(cookies.len())]
    );

    let result = init_server(&cookies, room_id);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_channel::mpsc::channel;

    #[test]
    fn test_bili_live_client_connect() {
        // Always enable debug log output for test
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
        // Get cookies from environment variable for real test
        let cookies =
            std::env::var("Cookie").unwrap_or_else(|_| "SESSDATA=dummy_sessdata".to_string());
        let room_id = "24779526";
        let (tx, _rx) = channel(10);
        let _client = BiliLiveClient::new(&cookies, room_id, tx);
    }
}
