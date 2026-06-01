# AGENTS.md - blivedm_rs Development Guide

## Project Overview

**blivedm_rs** is a Rust library/CLI for monitoring Bilibili live room danmaku via WebSocket, featuring TTS, auto-reply, browser cookie detection, and an interactive TUI.

## Architecture

```
WebSocket Client --> Event Scheduler --> Plugins
                                          ├── Terminal Display
                                          ├── TTS
                                          ├── Auto Reply
                                          └── Custom Handlers
```

## Module Structure

| Module      | Location        | Purpose                                     |
| ----------- | --------------- | ------------------------------------------- |
| **client**  | `src/client/`   | WebSocket connection, auth, message parsing |
| **plugins** | `src/plugins/`  | Event handlers (TTS, auto-reply, display)   |
| **tui**     | `src/tui/`      | Terminal UI with ratatui                    |
| **config**  | `src/config.rs` | TOML configuration parsing                  |

### Key Files

- `src/client/websocket.rs` - WebSocket client, Brotli decompression
- `src/client/auth.rs` - WBI signing, cookie/SESSDATA handling
- `src/client/models.rs` - `BiliMessage`, `AuthMessage` data structures
- `src/client/scheduler.rs` - `EventHandler` trait, event dispatch system
- `src/client/browser_cookies.rs` - Extract cookies from browsers
- `src/plugins/tts.rs` - TTS with REST API or local commands
- `src/plugins/auto_reply.rs` - Keyword-triggered auto-reply
- `src/tui/app.rs` - TUI state management

## NOTE

- if context is from GITHUB, first try use 'gh' CLI to get
