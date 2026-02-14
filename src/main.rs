// src/main.rs
// Standalone binary to test integration of the terminal display plugin with the BiliLiveClient

mod config;

use blivedm::client::get_cookies_or_browser;
use blivedm::client::scheduler::{EventContext, Scheduler};
use blivedm::client::websocket::BiliLiveClient;
use blivedm::plugins::terminal_display::TerminalDisplayHandler;
use blivedm::plugins::tts::TtsHandler;
use blivedm::tui::{TuiApp, TuiLogger, run_tui};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use config::Config;
use futures::channel::mpsc;
use futures::stream::StreamExt;
use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Print effective configuration and exit
    #[arg(long)]
    print_config: bool,
    /// Cookies for Bilibili authentication (optional - will auto-detect from browser if not provided)
    #[arg(long, value_name = "COOKIES")]
    cookies: Option<String>,

    /// Room ID to connect to
    #[arg(long, value_name = "ROOM_ID")]
    room_id: Option<String>,

    /// TTS REST API server URL
    #[arg(long, value_name = "URL")]
    tts_server: Option<String>,

    /// TTS voice ID (e.g., "zh-CN-XiaoxiaoNeural")
    #[arg(long, value_name = "VOICE")]
    tts_voice: Option<String>,

    /// TTS backend ("edge", "xtts", "piper")
    #[arg(long, value_name = "BACKEND")]
    tts_backend: Option<String>,

    /// TTS audio quality ("low", "medium", "high")
    #[arg(long, value_name = "QUALITY")]
    tts_quality: Option<String>,

    /// TTS audio format (e.g., "wav")
    #[arg(long, value_name = "FORMAT")]
    tts_format: Option<String>,

    /// TTS sample rate (e.g., 22050, 44100)
    #[arg(long, value_name = "RATE")]
    tts_sample_rate: Option<u32>,

    /// TTS audio volume (0.0 to 1.0)
    #[arg(long, value_name = "VOLUME")]
    tts_volume: Option<f32>,

    /// Local TTS command (e.g., "say", "espeak-ng")
    #[arg(long, value_name = "COMMAND")]
    tts_command: Option<String>,

    /// Comma-separated arguments for TTS command
    #[arg(long, value_name = "ARGS", allow_hyphen_values = true)]
    tts_args: Option<String>,

    /// Alibaba DashScope API key for ali-tts (can also use DASHSCOPE_API_KEY env)
    #[arg(long, value_name = "KEY")]
    ali_api_key: Option<String>,

    /// Alibaba TTS model (e.g., "qwen3-tts-flash")
    #[arg(long, value_name = "MODEL")]
    ali_model: Option<String>,

    /// Alibaba TTS voice (e.g., "Cherry", "Chelsie")
    #[arg(long, value_name = "VOICE")]
    ali_voice: Option<String>,

    /// Alibaba TTS language type (e.g., "Chinese", "English")
    #[arg(long, value_name = "LANG")]
    ali_language_type: Option<String>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Enable auto reply plugin
    #[arg(long)]
    auto_reply: bool,

    /// Generate shell completion script (bash, zsh, fish, powershell, elvish)
    #[arg(long, value_name = "SHELL")]
    generate_completion: Option<Shell>,
}

fn main() {
    let args = Args::parse();

    // Handle shell completion generation early (before any other processing)
    if let Some(shell) = args.generate_completion {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "blivedm", &mut std::io::stdout());
        return;
    }

    // Load configuration from file first
    let config = match Config::load_from_file(args.config.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize logging with precedence: CLI args > env vars > config file
    let debug_enabled =
        args.debug || env::var("DEBUG").unwrap_or_default() == "1" || config.debug.unwrap_or(false);

    // Load cookies and room_id with precedence: CLI args > env vars > config file > defaults
    let cookies = args
        .cookies
        .or_else(|| {
            env::var("Cookie")
                .ok()
                .filter(|s| !s.is_empty() && s != "SESSDATA=dummy_sessdata")
        })
        .or_else(|| config.connection.as_ref().and_then(|c| c.cookies.clone()));

    // If no manual cookies provided, try browser auto-detection
    let cookies = if cookies.is_none() {
        if debug_enabled {
            log::info!("No manual cookies provided, attempting browser auto-detection...");
        }
        get_cookies_or_browser(None)
    } else {
        if debug_enabled {
            log::info!("Using manually provided cookies");
        }
        cookies
    };

    let room_id = args
        .room_id
        .or_else(|| env::var("ROOM_ID").ok())
        .or_else(|| config.connection.as_ref().and_then(|c| c.room_id.clone()))
        .unwrap_or_else(|| "24779526".to_string());

    // Configure TTS with precedence: CLI args > config file
    let tts_server = args
        .tts_server
        .or_else(|| config.tts.as_ref().and_then(|t| t.server.clone()));
    let tts_voice = args
        .tts_voice
        .or_else(|| config.tts.as_ref().and_then(|t| t.voice.clone()));
    let tts_backend = args
        .tts_backend
        .or_else(|| config.tts.as_ref().and_then(|t| t.backend.clone()));
    let tts_quality = args
        .tts_quality
        .or_else(|| config.tts.as_ref().and_then(|t| t.quality.clone()));
    let tts_format = args
        .tts_format
        .or_else(|| config.tts.as_ref().and_then(|t| t.format.clone()));
    let tts_sample_rate = args
        .tts_sample_rate
        .or_else(|| config.tts.as_ref().and_then(|t| t.sample_rate));
    let tts_volume = args
        .tts_volume
        .or_else(|| config.tts.as_ref().and_then(|t| t.volume));
    let tts_command = args
        .tts_command
        .or_else(|| config.tts.as_ref().and_then(|t| t.command.clone()));
    let tts_args = args
        .tts_args
        .or_else(|| config.tts.as_ref().and_then(|t| t.args.clone()));

    // Configure Alibaba TTS with precedence: CLI args > env vars > config file
    let ali_api_key = args
        .ali_api_key
        .or_else(|| env::var("DASHSCOPE_API_KEY").ok())
        .or_else(|| config.tts.as_ref().and_then(|t| t.ali_api_key.clone()));
    let ali_model = args
        .ali_model
        .or_else(|| config.tts.as_ref().and_then(|t| t.ali_model.clone()));
    let ali_voice = args
        .ali_voice
        .or_else(|| config.tts.as_ref().and_then(|t| t.ali_voice.clone()));
    let ali_language_type = args.ali_language_type.or_else(|| {
        config
            .tts
            .as_ref()
            .and_then(|t| t.ali_language_type.clone())
    });

    // Configure auto reply with precedence: CLI args > config file
    let auto_reply_config = if let Some(config_auto_reply) = &config.auto_reply {
        // Use config file settings, but allow CLI flag to override enabled
        let mut plugin_config = config_auto_reply.to_plugin_config();
        if args.auto_reply {
            plugin_config.enabled = true;
        }
        plugin_config
    } else {
        // No config file section, use defaults with CLI flag
        let mut default_config = blivedm::plugins::auto_reply::AutoReplyConfig::default();
        default_config.enabled = args.auto_reply;
        default_config
    };

    // If user wants to see config, print and exit
    if args.print_config {
        // Create a temporary config struct for display that reflects the effective settings
        let effective_auto_reply = if auto_reply_config.enabled {
            Some(config::AutoReplyConfig {
                enabled: auto_reply_config.enabled,
                cooldown_seconds: auto_reply_config.cooldown_seconds,
                triggers: auto_reply_config
                    .triggers
                    .iter()
                    .map(|t| config::TriggerConfig {
                        keywords: t.keywords.clone(),
                        response: t.response.clone(),
                    })
                    .collect(),
            })
        } else {
            None
        };

        Config::print_effective_config(
            &cookies,
            &room_id,
            &tts_server,
            &tts_voice,
            &tts_backend,
            &tts_quality,
            &tts_format,
            &tts_sample_rate,
            &tts_volume,
            &tts_command,
            &tts_args,
            &ali_api_key,
            &ali_model,
            &ali_voice,
            &ali_language_type,
            &effective_auto_reply,
            debug_enabled,
        );
        std::process::exit(0);
    }

    // Initialize TuiLogger to capture logs into a shared buffer for the TUI logs panel.
    // When debug is enabled, capture Debug level; otherwise capture Info level.
    let log_level = if debug_enabled {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let log_buffer = TuiLogger::init(log_level);

    // Create client with automatic browser cookie detection
    let (tx, mut rx) = mpsc::channel(64);
    let mut client = match BiliLiveClient::new_auto(cookies.as_deref(), &room_id, tx) {
        Ok(client) => {
            log::info!("Successfully created client with automatic cookie detection");
            client
        }
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            eprintln!(
                "Please ensure you are logged into bilibili.com in your browser, or provide cookies manually."
            );
            std::process::exit(1);
        }
    };
    client.send_auth();
    client.send_heart_beat();
    let shared_client: Arc<Mutex<BiliLiveClient>> = Arc::new(Mutex::new(client));
    let heart_beats: Arc<Mutex<BiliLiveClient>> = Arc::clone(&shared_client);

    thread::spawn(move || {
        loop {
            match heart_beats.lock() {
                Ok(mut heart_beats_c) => {
                    heart_beats_c.send_heart_beat();
                }
                Err(e) => {
                    eprintln!("Error acquiring lock on stream: {}", e);
                    break;
                }
            }
            thread::sleep(Duration::new(20, 0));
        }
    });

    let rec_msg: Arc<Mutex<BiliLiveClient>> = Arc::clone(&shared_client);
    thread::spawn(move || {
        loop {
            match rec_msg.lock() {
                Ok(mut rec_c) => {
                    if let Err(e) = rec_c.receive() {
                        log::error!("{}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error acquiring lock on stream: {}", e);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10)); // instead of 10 microseconds
        }
    });

    // Set up the scheduler with context and add the terminal display handler
    if debug_enabled {
        match &cookies {
            Some(cookie_str) => {
                log::debug!(
                    "Cookies found and passed to context: {}...",
                    &cookie_str.chars().take(50).collect::<String>()
                );
            }
            None => {
                log::warn!(
                    "No cookies found for EventContext - auto-reply will not be able to send messages"
                );
            }
        }
    }

    // Create shared message buffer for TUI
    let message_buffer: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

    // Create shared online count for TUI title display
    let online_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

    let context = EventContext::new(cookies.clone(), room_id.parse::<u64>().unwrap_or(0));
    let mut scheduler = Scheduler::new(context);
    let terminal_handler = Arc::new(TerminalDisplayHandler::with_online_count(
        Arc::clone(&message_buffer),
        Arc::clone(&online_count),
    ));
    scheduler.add_sequential_handler(terminal_handler);
    if let Some(server_url) = tts_server {
        // REST API TTS configuration
        let tts_handler = Arc::new(TtsHandler::new_rest_api_with_volume(
            server_url,
            tts_voice,
            tts_backend,
            tts_quality,
            tts_format,
            tts_sample_rate,
            tts_volume,
        ));
        scheduler.add_sequential_handler(tts_handler);
        println!("TTS configured with REST API server");
    } else if let Some(api_key) = ali_api_key {
        // Alibaba DashScope TTS configuration
        let model = ali_model.unwrap_or_else(|| "qwen3-tts-flash".to_string());
        let voice = ali_voice.unwrap_or_else(|| "Cherry".to_string());
        let tts_handler = Arc::new(TtsHandler::new_ali_tts(
            api_key,
            model.clone(),
            voice.clone(),
            ali_language_type,
            tts_volume,
        ));
        scheduler.add_sequential_handler(tts_handler);
        println!(
            "TTS configured with Alibaba DashScope (model: {}, voice: {})",
            model, voice
        );
    } else if let Some(tts_cmd) = tts_command {
        // Command-line TTS configuration
        let cmd_args = tts_args
            .map(|s| s.split(',').map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let tts_handler = Arc::new(TtsHandler::new_command(tts_cmd, cmd_args));
        scheduler.add_sequential_handler(tts_handler);
        println!("TTS configured with local command");
    } else {
        println!(
            "No TTS configuration provided. Use --ali-api-key, --tts-server, or --tts-command to enable TTS."
        );
    }

    // Add auto reply plugin if enabled
    if auto_reply_config.enabled {
        let auto_reply_handler = blivedm::plugins::auto_reply_handler(auto_reply_config);
        scheduler.add_sequential_handler(auto_reply_handler);
        println!("Auto reply plugin enabled");
    } else {
        println!(
            "Auto reply plugin disabled. Use --auto-reply or configure in config file to enable."
        );
    }

    // Add initial system message to buffer
    TuiApp::add_message(&message_buffer, format!("[System] Bilibili Danmu Client"));
    TuiApp::add_message(
        &message_buffer,
        format!("[System] Connected to room: {}", room_id),
    );
    if let Some(cookies_val) = &cookies {
        TuiApp::add_message(
            &message_buffer,
            format!(
                "[System] Using provided cookies: {}...",
                &cookies_val.chars().take(30).collect::<String>()
            ),
        );
    } else {
        TuiApp::add_message(
            &message_buffer,
            "[System] Using auto-detected cookies from browser".to_string(),
        );
    }

    // create a thread to process the rx channel messages using tokio runtime and pass to scheduler
    let rt = Arc::new(Runtime::new().unwrap());
    let rt_clone = Arc::clone(&rt);
    rt.spawn(async move {
        while let Some(msg) = rx.next().await {
            scheduler.trigger(msg);
        }
    });

    // Create TUI app
    let mut tui_app = TuiApp::with_online_count(
        Arc::clone(&message_buffer),
        room_id.clone(),
        Arc::clone(&online_count),
    );
    tui_app.set_log_buffer(log_buffer);

    let context_for_chat = EventContext::new(cookies.clone(), room_id.parse::<u64>().unwrap_or(0));
    let message_buffer_for_feedback = Arc::clone(&message_buffer);

    // Run TUI with message sending callback
    let tui_result = run_tui(tui_app, move |message| {
        let context_clone = context_for_chat.clone();
        let rt_for_send = Arc::clone(&rt_clone);
        let buffer_clone = Arc::clone(&message_buffer_for_feedback);

        rt_for_send.spawn(async move {
            if let Err(e) = blivedm::plugins::send_danmaku_message(&message, &context_clone).await {
                TuiApp::add_message(
                    &buffer_clone,
                    format!("[System] Error sending message: {}", e),
                );
            }
        });
    });

    if let Err(e) = tui_result {
        eprintln!("TUI error: {}", e);
    }

    // close the client
    match shared_client.lock() {
        Ok(mut _client) => {}
        Err(e) => {
            eprintln!("Error acquiring lock on stream: {}", e);
        }
    }
    // wait for the threads to finish
    thread::sleep(Duration::new(1, 0));
}
