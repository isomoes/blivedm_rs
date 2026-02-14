# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Full-screen scrollable logs panel in TUI (Ctrl+L toggle)

## [0.5.3] - 2026-02-05

### Added
- **Ali TTS mode** - New `ali-tts` TTS backend using Alibaba DashScope API
- **Optional `browser-cookies` feature** to reduce dependencies when used as a library
- TTS priority: edge-tts is now preferred over other backends by default

### Fixed
- TUI text input not visible in light terminal themes
- Ali TTS streaming error handling

## [0.5.2] - 2026-01-21

### Added
- **Raw message toggle display in TUI** - View raw message data for debugging
- **Online count message category** (`BiliMessage`) for tracking viewer count

### Fixed
- Library usability improvements for downstream consumers (#23)
- Read message error no longer causes the danmaku connection to break (#11)

## [0.5.1] - 2025-11-24

### Added
- **🐚 Shell auto-completion support**
  - Generate completion scripts for bash, zsh, fish, elvish, and PowerShell
  - New `--generate-completion <SHELL>` CLI option
- **🦊 Firefox Snap and Flatpak support on Linux**
  - Browser cookie detection now works with Firefox installed via Snap
  - Browser cookie detection now works with Firefox installed via Flatpak

### Changed
- Updated README.md documentation with bilibili video formatting improvements

## [0.5.0] - 2025-10-25

### Added
- **✨ Interactive TUI (Terminal User Interface) with ratatui**
  - Full-featured terminal UI with scrollable message display
  - Real-time danmaku updates in a dedicated message area
  - Interactive input mode for sending danmaku messages
  - Status bar showing room ID and connection info
  - Keyboard shortcuts: Enter to send, Esc to quit
  - Shared message buffer architecture for seamless UI updates
- New `src/tui/` module with app state, event handling, and UI rendering
- Dependencies: ratatui 0.29, crossterm 0.28

### Changed
- Refactored TerminalDisplayHandler to use shared message buffer for TUI integration
- Updated main.rs to integrate TUI event loop with existing WebSocket client
- Enhanced user experience with non-blocking message input and display

### Fixed
- 🐛 Fix Chinese character input causing panic in TUI
  - Properly handle multi-byte UTF-8 characters (Chinese, Japanese, etc.)
  - Fixed cursor position calculation for display width of multi-byte characters
  - Added `unicode-width` dependency for accurate cursor positioning
  - Character insertion, deletion, and cursor movement now work correctly with non-ASCII text

## [0.4.6] - 2025-10-08

### Changed
- 📝 Update README.md formatting and remove precompiled binary reference

### Removed
- Remove Nix flake configuration files (flake.nix, flake.lock)
- Remove TTS_USAGE.md documentation file

## [0.4.5] - 2025-10-08

### Fixed
- 🔧 Fix GitHub Actions workflow permissions for creating releases
- Add `contents: write` permission to job level and workflow level

## [0.4.4] - 2025-10-08

### Fixed
- 🔧 Add system dependencies (libasound2-dev, pkg-config, libssl-dev) to GitHub Actions workflow
- Fix `cargo publish` failing due to missing ALSA libraries in CI

## [0.4.3] - 2025-10-08

### Fixed
- 🔧 Update CLAUDE.md release process to include `cargo update -p blivedm` step
- Fix Cargo.lock synchronization for CI/CD pipeline

## [0.4.2] - 2025-10-08

### Changed
- ♻️ **Consolidate workspace into single package structure**
  - Merged `client`, `plugins`, and `examples` packages into root package
  - Moved all modules into `src/client/` and `src/plugins/` directories
  - Added library target (`src/lib.rs`) exporting all functionality
  - Converted binaries to examples (`examples/` directory)
  - Simplified dependency management with single Cargo.toml
  - Updated GitHub release workflow to publish to crates.io with OIDC authentication
  - Removed duplicate dependencies and streamlined build configuration
- Updated CLAUDE.md documentation to reflect single-package architecture

### Removed
- Workspace structure (client/, plugins/, examples/ packages)
- Redundant Cargo.toml files in subdirectories
- `tokio-native-tls` and `hyper-tls` dependencies in favor of rustls

## [0.4.0] - 2025-08-16

### Added
- **🤖 Auto reply plugin with keyword detection and Bilibili API integration**
  - Keyword-based trigger system with configurable responses
  - Rate limiting with configurable cooldown periods
  - HTTP client integration with Bilibili msg/send API
  - TOML configuration support with CLI override
  - Cookie passing mechanism for authentication
  - Robust error handling and logging
- EventContext system for passing cookies and room_id to plugins
- `[auto_reply]` section in TOML configuration files
- `--auto-reply` CLI flag to enable auto reply plugin
- `get_cookies_or_browser` function exposed in public API
- EventContext helper constructors (`new()`, `new_with_auto_cookies()`)

### Changed
- Extended EventHandler trait to accept EventContext parameter
- Updated all existing plugins (terminal_display, tts) to support new EventHandler signature
- Improved cookie detection with better debug logging
- Enhanced main application configuration precedence handling

### Fixed
- Fixed examples to work with new EventHandler trait signature

## [0.3.5] - 2025-08-10

### Added
- Comprehensive TOML configuration file support
- Configuration file auto-creation with examples
- Support for multiple configuration file locations
- Configuration precedence: CLI args > env vars > config file > defaults

### Changed
- Updated README.md with TOML configuration documentation

### Fixed
- Remove duplicate workflow triggers
- Add missing Linux system dependencies for ALSA

## [0.3.4] - 2025-08-10

### Added
- Comprehensive CI/CD with native Windows compilation
- Claude Code configuration and MCP settings

### Changed
- Comprehensive README update with enhanced features and reorganization
- Remove GitHub Pages documentation references
- Remove old README.zh.md as Chinese is now main README.md

### Fixed
- Update .claude submodule reference

## [0.3.3] - 2025-08-03

### Changed
- Update Windows TTS instructions and note about PowerShell compatibility

## [0.3.2] - 2025-07-22

### Fixed
- Allow hyphen values in tts-args parameter

## [0.3.1] - 2025-06-28

### Added
- Chinese version of README (README.zh.md)
- Link to Chinese README in main README
- Note about detailed usage guide in Quick Start section
- GitHub issue management commands to CLAUDE.md
- CLAUDE.md for project guidance and development instructions

### Changed
- Update CLI usage examples to use --room-id flag format
- Update danmu client run instructions to support cookie auto-detection and manual cookie input
- Update authentication to use cookies instead of SESSDATA on CLI

### Fixed
- Remove unused browser cookie test example

## [0.3.0] - 2025-06-28

### Added
- Enhanced cookie handling with browser cookie detection
- Automatic SESSDATA retrieval logic that prioritizes browser cookies
- Example HTTP request for Danmu info retrieval with dynamic cookie variable

### Changed
- Improve SESSDATA retrieval logic to prioritize browser cookies
- Update example HTTP request format for Danmu info retrieval

### Fixed
- getDanmuInfo WebSocket server `-352` error code issue (#3)

## [0.2.5] - 2025-06-18

### Added
- Automatic browser cookie detection for SESSDATA authentication

### Fixed
- Logined SESSDATA timeout causing unlogin issue (#4)

## [0.2.4] - 2025-06-14

### Added
- Volume control to TTS handler with new REST API methods
- Dual-mode TTS support with REST API and command-line options
- TTS functionality using danmu-tts REST API
- Enhanced TTS configuration with additional command-line arguments:
  - Quality, format, sample rate, volume, and command options

### Changed
- Update danmu documentation to enhance TTS configuration details and usage examples
- Update README to clarify building from source and system requirements
- Clean up imports and improve code formatting in TTS handler
- Update TtsMetadata fields to be optional and handle potential None values

### Fixed
- Remove outdated TTS setup link from README documentation
- Remove outdated TTS setup documentation and update sidebar
- Clean up release workflow by removing build job and artifact handling

## [0.2.3] - 2025-06-13

### Added
- TTS functionality using danmu-tts REST API
- Comprehensive TTS documentation

### Fixed
- LLM TTS Service integration (#6)

## [0.2.2] - 2025-06-02

### Fixed
- Remove integration_bili_live_client binary from artifacts upload

## [0.2.1] - Earlier releases

### Added
- Basic TTS support
- Terminal display plugin
- WebSocket client for Bilibili live rooms
- Browser cookie detection
- Plugin architecture

## [0.2.0] - Earlier releases

### Added
- Plugin system architecture
- Scheduler for event handling
- Multiple plugin support

## [0.1.0] - Initial Release

### Added
- Basic Bilibili live room danmaku client
- WebSocket connection to Bilibili live rooms
- Terminal output for danmaku messages
- Authentication system
- Basic configuration support

---

## Links

- [Issue #16: Add auto replay](https://github.com/jiahaoxiang2000/blivedm_rs/issues/16)
- [Issue #10: Easy configure the TTS plugins](https://github.com/jiahaoxiang2000/blivedm_rs/issues/10)
- [Issue #6: Add LLM TTS Service](https://github.com/jiahaoxiang2000/blivedm_rs/issues/6)
- [Issue #4: SESSDATA timeout issue](https://github.com/jiahaoxiang2000/blivedm_rs/issues/4)
- [Issue #3: getDanmuInfo WebSocket server error](https://github.com/jiahaoxiang2000/blivedm_rs/issues/3)