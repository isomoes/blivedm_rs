# blivedm_rs

一个功能强大的 Bilibili 直播间弹幕 WebSocket 客户端 Rust 库，支持实时弹幕监控、文字转语音（TTS）和浏览器 Cookie 自动检测。

[English Version README](README.en.md)

**Bilibili Video** [start point](https://www.bilibili.com/video/BV1F4jizbEGV) [v0.4](https://www.bilibili.com/video/BV1EvbtzNEbj) [v0.5](https://www.bilibili.com/video/BV1gXsZz8EgZ)

<img width="3808" height="2031" alt="Image" src="https://github.com/user-attachments/assets/c97afceb-ca3a-4a54-b450-b23bc3fe2275" />

## 🚀 主要功能

- **🖥️ 交互式 TUI 界面** - **v0.5.0 新功能！** 基于 ratatui 的终端用户界面，可滚动消息显示和交互式输入
- **🤖 智能自动回复** - **v0.4.0 新功能！** 基于关键词检测的自动回复系统，支持 Bilibili API 集成
- **🔍 智能 Cookie 检测** - 自动从主流浏览器（Chrome、Firefox、Edge、Opera）检测登录状态
- **💬 实时弹幕监控** - 连接 Bilibili 直播间，实时接收弹幕、礼物、进房等消息
- **🔊 多平台 TTS 支持** - 支持本地 TTS（Windows PowerShell、macOS say、Linux espeak-ng）和远程 TTS 服务器
- **🎛️ 插件化架构** - 模块化设计，支持自定义插件扩展功能
- **🖥️ 跨平台支持** - 原生支持 Windows、macOS、Linux
- **⚡ 高性能异步** - 基于 Tokio 的异步架构，低资源占用，高并发处理
- **🔧 灵活配置** - 支持 TOML 配置文件和命令行参数，可自定义 TTS 音量、语音等参数

## ✨ 自动浏览器 Cookie 检测

**无需手动提取 Cookie！** 客户端现在可以自动从您的浏览器检测 bilibili cookies。

支持 Linux、macOS 和 Windows 上的 Chrome、Firefox、Edge、Chromium 和 Opera。详情请参阅[浏览器 Cookie 文档](docs/browser-cookies.md)。

## 快速开始

### 安装方式

#### 从 crates.io 安装（推荐）

如果您已安装 Rust，可以直接从 crates.io 安装：

```bash
cargo install --locked blivedm
```

安装后即可在任何位置使用 `blivedm` 命令。

#### 从源码本地构建安装

```bash
# 克隆仓库
git clone https://github.com/jiahaoxiang2000/blivedm_rs.git
cd blivedm_rs

# 构建并安装到本地
cargo install --locked --path .
```

安装后即可在任何位置使用 `blivedm` 命令。

### 使用示例

```bash
# v0.4.0 新功能：启用自动回复插件
blivedm --room-id 12345 --auto-reply

# 自动检测浏览器 cookies（推荐）
blivedm --room-id 24779526

# 使用配置文件
blivedm --config config.toml

# 查看有效配置
blivedm --print-config

# 手动 cookies（必须包含 SESSDATA）
blivedm --cookies "SESSDATA=your_sessdata; other_cookie=..." --room-id 12345

# 使用 TTS REST API 服务器
blivedm --room-id 12345 --tts-server http://localhost:8000 --tts-volume 0.7

# 使用本地 TTS（macOS）
blivedm --room-id 12345 --tts-command say --tts-args "-v,Mei-Jia"

# 使用本地 TTS（Linux）
blivedm --room-id 12345 --tts-command espeak-ng --tts-args "-v,cmn"

# ⚠️ Windows 用户建议：使用 TTS 服务器获得更好的语音体验
# 本地 PowerShell TTS 存在技术限制，推荐使用远程 TTS 服务器：
blivedm --room-id 12345 --tts-server http://localhost:8000

# 显示所有可用选项
blivedm --help
```

### TTS 服务器设置（Windows 用户推荐）

**Windows 用户特别推荐使用 TTS 服务器！** 相比受限的本地 PowerShell TTS，服务器提供更好的语音质量和功能。

```bash
# 克隆并设置 TTS 服务器
git clone https://github.com/jiahaoxiang2000/danmu-tts.git
cd danmu-tts
# 按照仓库中的设置说明进行操作
```

**TTS 服务器优势：**

- 🎙️ **高质量语音** - 支持神经网络 TTS 和多种语音引擎
- 🌐 **多语言支持** - 支持中文、英文等多种语言
- ⚙️ **灵活配置** - 可自定义语音参数、音调、语速
- 🔧 **易于部署** - 独立运行，无需复杂配置

详细设置说明请参阅 [danmu-tts 仓库](https://github.com/jiahaoxiang2000/danmu-tts)。

## 📋 配置文件支持

**新功能！** 现在支持 TOML 配置文件，简化复杂命令行参数：

### 配置文件位置

客户端按以下顺序查找配置文件：

1. `--config` 指定的路径
2. 当前目录的 `config.toml`
3. `~/.config/blivedm_rs/config.toml`（XDG 配置目录）

### 配置文件格式

创建 `config.toml` 文件：

```toml
# 连接设置
[connection]
room_id = "24779526"
# cookies = "SESSDATA=your_sessdata; other_cookie=value"

# v0.4.0 新功能：自动回复配置
[auto_reply]
enabled = true
triggers = [
    { keywords = ["测试", "test"], response = "这是一个测]试回复" },
]
cooldown_seconds = 3

# TTS 配置
[tts]
server = "http://localhost:8000"
voice = "zh-CN-XiaoxiaoNeural"
backend = "edge"
quality = "medium"
format = "wav"
sample_rate = 22050
volume = 0.8

# 或使用本地 TTS 命令
# command = "say"
# args = "--voice=Kyoko"

# 调试日志
debug = false
```

### 配置优先级

配置值按以下优先级解析（从高到低）：

1. **命令行参数**（最高优先级）
2. **环境变量**（如 `ROOM_ID`、`Cookie`）
3. **配置文件**值
4. **默认值**（最低优先级）

### 配置相关命令

```bash
# 使用指定配置文件
blivedm --config path/to/config.toml

# 查看有效配置（调试用）
blivedm --print-config

# 命令行参数覆盖配置文件
blivedm --config config.toml --room-id 12345 --debug

# v0.4.0 新功能：启用自动回复功能
blivedm --config config.toml --auto-reply
```

**提示：** 如果未找到配置文件，程序会自动创建示例配置文件 `config.toml.example` 供参考。

## 从源码构建

如果您希望从源码构建或进行开发，可以按照以下步骤：

```bash
# 克隆仓库
git clone https://github.com/jiahaoxiang2000/blivedm_rs.git
cd blivedm_rs

# 构建项目
cargo build --release

# 运行弹幕客户端（自动检测浏览器 cookies）
./target/release/blivedm --room-id 24779526

# 使用配置文件
./target/release/blivedm --config config.toml

# 或使用手动 cookies（必须包含 SESSDATA）
./target/release/blivedm --cookies "SESSDATA=your_sessdata; other_cookie=..." --room-id 24779526
```

### 系统要求

- **Rust**: 最新稳定版本
- **Linux**:
  - 音频支持：`sudo apt-get install libasound2-dev`
  - 构建工具：`sudo apt-get install pkg-config libssl-dev`
  - 可选 TTS：`sudo apt-get install espeak-ng`
- **macOS**: 无需额外依赖（使用内置 `say` 命令进行 TTS）
- **Windows**: 无需额外依赖（通过 System.Speech 使用内置 PowerShell TTS）

详细使用指南请参阅 [Danmu](docs/danmu.md) 页面。

## 文档

完整文档位于 [`docs/`](docs/) 文件夹中。主要部分包括：

- [入门指南](docs/README.md)：介绍和设置说明。
- [浏览器 Cookie 自动检测](docs/browser-cookies.md)：**新功能！** 自动 cookie 检测的工作原理。
- [使用指南](docs/usage.md)：如何在您的项目中使用该库。
- [架构](docs/architecture.md)：项目架构和设计。
- [客户端模块](docs/client.md)：客户端实现的详细信息。
- [弹幕模块](docs/danmu.md)：弹幕模块的信息。
- [调度器](docs/scheduler.md)：调度器组件概述。
- [插件](docs/plugins.md)：可用插件及其使用方法。

## 参考

- [blivedm](https://github.com/xfgryujk/blivedm)：Bilibili 直播弹幕协议的原始 Python 实现。
- [blivedm-rs](https://github.com/yanglul/blivedm_rs)：blivedm 库的 Rust 移植版。
- [bililive-rs](https://github.com/LightQuantumArchive/bililive-rs)：另一个用于 Bilibili 直播的 Rust 实现。
- [bilibili-API-collect](https://github.com/SocialSisterYi/bilibili-API-collect)：SocialSisterYi 的 Bilibili API 集合。

## Star History

<a href="https://star-history.com/#isomoes/blivedm_rs&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=isomoes/blivedm_rs&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=isomoes/blivedm_rs&type=Date" />
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=isomoes/blivedm_rs&type=Date" />
  </picture>
</a>
