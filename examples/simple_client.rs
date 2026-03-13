use blivedm::client::models::BiliMessage;
use blivedm::client::websocket::BiliLiveClient;
use futures::channel::mpsc;
use futures::stream::StreamExt;
use std::env;

#[tokio::main]
async fn main() {
    let room_id = "24779526"; // A popular room
    let (tx, mut rx) = mpsc::channel(32);

    // We'll just use a dummy cookie or empty one for testing connection
    let cookies = env::var("Cookie").ok();

    println!("Connecting to room {}", room_id);

    let cookies_clone = cookies.clone();
    let room_id_clone = room_id.to_string();
    let tx_clone = tx.clone();

    // BiliLiveClient::new_auto is blocking, so we spawn it in a blocking task
    let client = tokio::task::spawn_blocking(move || {
        BiliLiveClient::new_auto(cookies_clone.as_deref(), &room_id_clone, tx_clone).unwrap()
    })
    .await
    .unwrap();

    let mut client = client;
    client.send_auth();
    client.send_heart_beat();

    // Spawn a thread to keep sending heartbeats
    let client_clone = std::sync::Arc::new(std::sync::Mutex::new(client));
    let hb_client = client_clone.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Ok(mut c) = hb_client.lock() {
                c.send_heart_beat();
            }
        }
    });

    // Spawn a thread to receive messages from websocket
    let recv_client = client_clone.clone();
    tokio::spawn(async move {
        loop {
            let res = {
                // receive() is blocking
                let client_lock = recv_client.clone();
                tokio::task::spawn_blocking(move || {
                    if let Ok(mut c) = client_lock.lock() {
                        c.receive()
                    } else {
                        Err("Lock error".to_string())
                    }
                })
                .await
                .unwrap()
            };

            if let Err(e) = res {
                eprintln!("Receive error: {}", e);
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    println!("Listening for messages...");
    while let Some(msg) = rx.next().await {
        match msg {
            BiliMessage::Danmu { user, text } => {
                println!("Danmu: {}: {}", user, text);
            }
            BiliMessage::Gift { user, gift, .. } => {
                println!("Gift: {} sent {}", user, gift);
            }
            BiliMessage::Raw(json) => {
                println!("Raw: {:?}", json);
            }
            _ => {}
        }
    }
}
