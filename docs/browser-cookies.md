# Browser Cookie Auto-Detection

## Overview

The `blivedm_rs` library now supports automatic detection of SESSDATA cookies from your browser, eliminating the need to manually extract and provide authentication cookies.

## Features

- **Multi-browser support**: Automatically detects cookies from Chrome, Firefox, Edge, Chromium, and Opera
- **Cross-platform**: Works on Linux, macOS, and Windows
- **Automatic fallback**: Uses manual SESSDATA if provided, otherwise searches browser cookies
- **Cookie validation**: Checks expiration and verifies browser login with Bilibili
- **Safe operation**: Creates temporary copies of browser databases to avoid conflicts

## Usage

### Encrypted Chrome cookies on Linux

Chromium `v10` and `v11` cookies are decrypted in memory, including the host
binding used by cookie database version 24 and later. For `v11`, install
`secret-tool` (the `libsecret` package on Arch Linux, or `libsecret-tools` on
Debian/Ubuntu) and unlock the desktop keyring used by Chrome. The lookup times
out if the keyring is unavailable. Cookie values and keyring secrets are not
printed by cookie detection.

Encrypted Chromium cookies on macOS and Windows are not decrypted by this
implementation. Use Firefox or provide cookies manually on those platforms.
Sending requires both a valid `SESSDATA` and a nonempty `bili_jct` CSRF cookie.

Auto-detection checks browsers in this order: Chrome, Firefox, Edge, Chromium,
Opera. For each browser with a nonempty, unexpired `SESSDATA`, it verifies login
using Bilibili's read-only `/x/web-interface/nav` API. Logged-out sessions are
skipped, and the first verified login is selected. Cookies from different
browsers are never combined. Explicit cookies take precedence over detection
and are not automatically replaced with another account.

Login checks have a 5-second connection timeout and a 10-second total timeout
per browser. Network errors, malformed responses, and API failures are reported
separately from logout; detection continues to the next browser. If no login
can be verified, no browser session is selected. No live messages are sent to
validate login. This check runs during auto-detection, not continuously while
the client is running.

### Automatic Detection (Recommended)

Simply run the client without providing SESSDATA:

```bash
# No SESSDATA needed - will auto-detect from browser
cargo run -- --room-id 24779526

# With debug logging to see detection process
cargo run -- --room-id 24779526 --debug
```

### Manual SESSDATA (Still Supported)

You can still provide SESSDATA manually if needed:

```bash
# Provide SESSDATA explicitly
cargo run -- --room-id 24779526 --cookies "SESSDATA=your_sessdata_here"

# Or via environment variable (still need --room-id flag)
export SESSDATA="your_sessdata_here"
cargo run -- --room-id 24779526 --cookies "SESSDATA=$SESSDATA"
```

### Testing Cookie Detection

Use the provided test utility to check what cookies are available:

```bash
# Note: browser_cookie_test binary was removed from source
# Cookie detection is now integrated into the main blivedm client
# Use --debug flag to see cookie detection details:
cargo run -- --room-id 24779526 --debug
```

This will show:
- Whether a valid SESSDATA cookie was found
- All bilibili cookies in your browsers
- Browser support status

## How It Works

1. **Browser Database Location**: The system knows where each browser stores cookies on different operating systems
2. **Safe Reading**: Creates temporary copies of cookie databases to avoid locking issues
3. **Cookie Parsing**: Reads SQLite databases (Chromium-based) or Firefox's cookie format
4. **Validation**: Checks for:
   - Cookie expiration dates
   - Minimum length requirements
   - Domain matching (bilibili.com)
5. **Fallback Chain**: Tries browsers in order until a valid cookie is found

## Supported Browsers

| Browser   | Linux | macOS | Windows | Status |
|-----------|-------|--------|---------|---------|
| Chrome    | ✅     | ✅      | ✅       | Supported |
| Firefox   | ✅     | ✅      | ✅       | Supported |
| Edge      | ✅     | ✅      | ✅       | Supported |
| Chromium  | ✅     | ✅      | ✅       | Supported |
| Opera     | ✅     | ✅      | ✅       | Supported |

## Cookie Storage Locations

### Linux
- Chrome: `~/.config/google-chrome/Default/Cookies`
- Firefox: `~/.mozilla/firefox/*/cookies.sqlite`
- Edge: `~/.config/microsoft-edge/Default/Cookies`
- Chromium: `~/.config/chromium/Default/Cookies`
- Opera: `~/.config/opera/Default/Cookies`

### macOS
- Chrome: `~/Library/Application Support/Google/Chrome/Default/Cookies`
- Firefox: `~/Library/Application Support/Firefox/Profiles/*/cookies.sqlite`
- Edge: `~/Library/Application Support/Microsoft Edge/Default/Cookies`
- Chromium: `~/Library/Application Support/Chromium/Default/Cookies`
- Opera: `~/Library/Application Support/com.operasoftware.Opera/Default/Cookies`

### Windows
- Chrome: `%LOCALAPPDATA%\Google\Chrome\User Data\Default\Network\Cookies`
- Firefox: `%APPDATA%\Mozilla\Firefox\Profiles\*\cookies.sqlite`
- Edge: `%LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Network\Cookies`
- Chromium: `%LOCALAPPDATA%\Chromium\User Data\Default\Network\Cookies`
- Opera: `%APPDATA%\Opera Software\Opera Stable\Network\Cookies`

## API Reference

### New Functions

```rust
use client::browser_cookies::{find_bilibili_sessdata, get_all_bilibili_cookies};

// Find SESSDATA cookie specifically
let sessdata = find_bilibili_sessdata();

// Get all bilibili cookies for debugging
let all_cookies = get_all_bilibili_cookies();
```

### Enhanced Client Creation

```rust
use client::websocket::BiliLiveClient;

// Automatic detection
let client = BiliLiveClient::new_auto(None, "24779526", tx)?;

// With optional manual SESSDATA
let client = BiliLiveClient::new_auto(Some("manual_sessdata"), "24779526", tx)?;

// Traditional method (still works)
let client = BiliLiveClient::new("manual_sessdata", "24779526", tx);
```

## Troubleshooting

### No Cookies Found
- Ensure you're logged into bilibili.com in your browser
- Check that the browser is properly installed
- Verify you have read permissions for the browser's data directory

### Invalid/Expired Cookies
- Log out and log back into bilibili.com
- Clear bilibili cookies and log in again
- Check if your account is still active

### Permission Issues
- On Linux/macOS: Ensure the browser isn't running when accessing cookies
- On Windows: Run as administrator if needed
- Check file permissions on browser data directories

### Debug Information
Use `--debug` flag to see detailed information about the cookie detection process:

```bash
cargo run -- --room-id 24779526 --debug
```

This will show:
- Which browsers are being checked
- Cookie database paths
- Authentication success/failure
- Detailed error messages

## Security Notes

- The system only reads cookies, never modifies them
- Temporary copies are created and cleaned up automatically
- No cookie data is stored or transmitted beyond what's needed for bilibili authentication
- Only bilibili.com domain cookies are accessed

## Limitations

- Requires read access to browser cookie databases
- Some browsers may lock databases when running (close browser if issues occur)
- Encrypted cookie stores (like Chrome on some systems) are not yet supported
- Only works with locally installed browsers (not portable versions)
