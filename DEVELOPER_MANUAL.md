# BlockAndFocus Developer Manual

> A comprehensive guide to understanding, operating, and modifying the BlockAndFocus DNS-level domain blocker for macOS.

**Target Audience:** Developers new to Rust, TypeScript, and systems programming.

---

## Table of Contents

- [Part 1: Operations Guide](#part-1-operations-guide)
  - [1.1 Quick Reference Card](#11-quick-reference-card)
  - [1.2 Starting the System](#12-starting-the-system)
  - [1.3 Stopping the System](#13-stopping-the-system)
  - [1.4 Production Mode](#14-production-mode)
  - [1.5 Troubleshooting Guide](#15-troubleshooting-guide)
- [Part 2: Concepts Explained](#part-2-concepts-explained)
  - [2.1 DNS Fundamentals](#21-dns-fundamentals)
  - [2.2 Ports and Sockets](#22-ports-and-sockets)
  - [2.3 launchd (macOS Service Manager)](#23-launchd-macos-service-manager)
  - [2.4 Dev vs Production Mode](#24-dev-vs-production-mode)
- [Part 3: Architecture Deep Dive](#part-3-architecture-deep-dive)
  - [3.1 System Overview](#31-system-overview)
  - [3.2 The Daemon](#32-the-daemon)
  - [3.3 The Tauri App](#33-the-tauri-app)
  - [3.4 Request Flows](#34-request-flows)
- [Part 4: Codebase Walkthrough](#part-4-codebase-walkthrough)
  - [4.1 Directory Structure](#41-directory-structure)
  - [4.2 Daemon Code](#42-daemon-code)
  - [4.3 App Code](#43-app-code)
  - [4.4 UI Code](#44-ui-code)
  - [4.5 Shared Types](#45-shared-types)
- [Part 5: Making Changes](#part-5-making-changes)
  - [5.1 Common Modifications](#51-common-modifications)
  - [5.2 Debugging Tips](#52-debugging-tips)
- [Part 6: Rust Crash Course](#part-6-rust-crash-course)
- [Part 7: TypeScript & Svelte Crash Course](#part-7-typescript--svelte-crash-course)

---

# Part 1: Operations Guide

This section covers everything you need to run, stop, and troubleshoot the system.

## 1.1 Quick Reference Card

### Essential Commands

| Command | What It Does |
|---------|--------------|
| `just daemon-dev` | Start the DNS daemon in dev mode (port 5454) |
| `just app-dev` | Start the Tauri UI app in dev mode |
| `just install-dev` | Install daemon as system service (requires sudo) |
| `just uninstall-dev` | Remove daemon from system |
| `just set-dns` | Point system DNS to 127.0.0.1 |
| `just restore-dns` | Restore system DNS to automatic |
| `just reload-daemon` | Rebuild and restart production daemon |

### Emergency Commands

If something goes wrong, run these in order:

```bash
# 1. Restore internet access immediately
sudo networksetup -setdnsservers Wi-Fi Empty
sudo dscacheutil -flushcache

# 2. Stop all BlockAndFocus processes
sudo pkill -9 -f blockandfocus-daemon
pkill -f "cargo tauri"
pkill -f "cargo run"

# 3. If production daemon keeps restarting, unload it
sudo launchctl bootout system /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# 4. Verify port 5454 is free
lsof -i :5454  # Should show nothing

# 5. Verify port 53 is free (may show mDNSResponder, that's OK)
sudo lsof -i :53
```

### Diagnostic Commands

```bash
# Check if daemon is running
pgrep -f blockandfocus-daemon

# Test DNS blocking directly (dev mode)
dig @127.0.0.1 -p 5454 facebook.com +short
# Should return: 0.0.0.0 (blocked) or real IP (allowed)

# Test DNS blocking (production mode)
dig @127.0.0.1 facebook.com +short

# Check what your system DNS is set to
networksetup -getdnsservers Wi-Fi

# Check if IPC socket exists
ls -la /tmp/blockandfocus-dev.sock      # Dev socket
ls -la /var/run/blockandfocus.sock      # Production socket

# Send ping to daemon via IPC
echo '{"type":"Ping"}' | nc -U /tmp/blockandfocus-dev.sock

# Get daemon status via IPC
echo '{"type":"GetStatus"}' | nc -U /tmp/blockandfocus-dev.sock
```

## 1.2 Starting the System

### Development Mode (Recommended for Development)

Development mode runs everything without root privileges and uses a non-standard port (5454) so it doesn't interfere with your system's actual DNS.

**Step 1: Start the Daemon**

Open Terminal 1:
```bash
cd /Users/niccologrillo/BlockAndFocus
just daemon-dev
```

You should see output like:
```
Starting daemon in development mode...
2025-11-30T21:11:07.856798Z  INFO blockandfocus_daemon: BlockAndFocus daemon starting...
2025-11-30T21:11:07.856890Z  INFO blockandfocus_daemon: Running in development mode
2025-11-30T21:11:07.857147Z  INFO blockandfocus_daemon::config::loader: Loading config from ./config.toml
2025-11-30T21:11:07.859754Z  INFO blockandfocus_daemon: Configuration loaded
2025-11-30T21:11:07.860233Z  INFO blockandfocus_daemon::dns::server: Starting DNS server on 127.0.0.1:5454
2025-11-30T21:11:07.860317Z  INFO blockandfocus_daemon::dns::server: DNS server listening on 127.0.0.1:5454
2025-11-30T21:11:07.860567Z  INFO blockandfocus_daemon::ipc::server: Starting IPC server on /tmp/blockandfocus-dev.sock
```

**What this command does under the hood:**
1. Sets environment variable `BLOCKANDFOCUS_DEV=1`
2. Sets `RUST_LOG=debug` for verbose logging
3. Runs `cargo run --package blockandfocus-daemon`
4. Daemon reads config from `./config.toml` (local file, not system file)
5. Listens for DNS queries on port 5454
6. Creates IPC socket at `/tmp/blockandfocus-dev.sock`

**Step 2: Start the UI App**

Open Terminal 2:
```bash
cd /Users/niccologrillo/BlockAndFocus
just app-dev
```

You should see Vite start, then a window should appear with the BlockAndFocus UI.

**What this command does:**
1. Sets `BLOCKANDFOCUS_DEV=1`
2. Runs `cargo tauri dev`
3. Tauri starts Vite dev server for hot-reloading
4. App connects to `/tmp/blockandfocus-dev.sock` to talk to daemon
5. Opens a window (or system tray icon)

**Step 3: Verify Everything Works**

Test DNS blocking:
```bash
dig @127.0.0.1 -p 5454 facebook.com +short
# Expected: 0.0.0.0 (blocked)

dig @127.0.0.1 -p 5454 google.com +short
# Expected: Real IP like 142.250.200.142 (allowed)
```

**Important:** In dev mode, only `dig` queries to port 5454 go through your daemon. Your browser still uses the system DNS (usually port 53), so browser traffic is NOT affected by the dev daemon.

### Order Matters!

Always start the daemon FIRST, then the app. Here's why:

1. The app tries to connect to the daemon's IPC socket on startup
2. If the daemon isn't running, there's no socket to connect to
3. The app will show "Daemon not connected" error

## 1.3 Stopping the System

### Clean Shutdown

**Development Mode:**
- Press `Ctrl+C` in each terminal (daemon terminal and app terminal)
- The daemon will log "Received shutdown signal" and exit cleanly
- The IPC socket file is automatically removed

**Production Mode:**
```bash
# Stop the daemon service
sudo launchctl bootout system /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# Optionally restore DNS
just restore-dns
```

### Force Kill (When Things Go Wrong)

If Ctrl+C doesn't work:
```bash
# Kill all daemon processes
sudo pkill -9 -f blockandfocus-daemon

# Kill Tauri/cargo processes
pkill -f "cargo tauri"
pkill -f "cargo run"
```

### What Happens When You Stop

**Dev Mode:**
- Nothing special - your system DNS continues working normally
- Browser traffic was never going through the dev daemon anyway

**Production Mode:**
- If your system DNS is set to 127.0.0.1, you'll lose internet access!
- Always run `just restore-dns` before stopping the production daemon
- Or keep the daemon running (it should auto-restart via launchd)

## 1.4 Production Mode

Production mode runs the daemon as a system service with root privileges on port 53 (the standard DNS port). This means ALL DNS traffic on your Mac goes through BlockAndFocus.

### Installing for Production

```bash
# Build the release binary
just build-daemon

# Install as system service (requires sudo)
just install-dev
```

**What `just install-dev` does:**
1. Copies the compiled daemon to `/Library/PrivilegedHelperTools/blockandfocus-daemon`
2. Copies the launchd plist to `/Library/LaunchDaemons/com.blockandfocus.daemon.plist`
3. Sets correct permissions (root:wheel ownership)
4. Loads the service with `launchctl load`
5. Daemon starts automatically and listens on port 53

### Activating Production Blocking

After installing, point your system DNS to the daemon:
```bash
just set-dns
```

This runs:
```bash
sudo networksetup -setdnsservers Wi-Fi 127.0.0.1
sudo dscacheutil -flushcache
```

Now ALL DNS queries go through BlockAndFocus!

### Updating Production Daemon

After making code changes:
```bash
just reload-daemon
```

This rebuilds the binary and restarts the service.

### Uninstalling

```bash
# First restore DNS
just restore-dns

# Then uninstall
just uninstall-dev
```

## 1.5 Troubleshooting Guide

### Problem: "Port already in use" Error

**Symptom:**
```
ERROR blockandfocus_daemon: DNS server error: Failed to bind DNS socket on 127.0.0.1:5454
```

**Cause:** Another process is using the port.

**Solution:**
```bash
# Find what's using the port
sudo lsof -i :5454

# Example output:
# COMMAND     PID USER   FD   TYPE  DEVICE SIZE/OFF NODE NAME
# blockandf 86376 root    9u  IPv4  ...    0t0  UDP localhost:apc-5454

# Kill the process
sudo kill -9 86376

# If it keeps coming back, it's launchd restarting it:
sudo launchctl bootout system /Library/LaunchDaemons/com.blockandfocus.daemon.plist
```

### Problem: "Daemon not connected" in UI

**Symptom:** The UI shows "The BlockAndFocus daemon is not running"

**Possible Causes:**

1. **Daemon not running:** Start it with `just daemon-dev`

2. **Socket doesn't exist:**
   ```bash
   ls -la /tmp/blockandfocus-dev.sock
   # If "No such file", daemon isn't running or crashed
   ```

3. **Wrong socket path:** If you're running the app without `BLOCKANDFOCUS_DEV=1`, it tries to connect to the production socket `/var/run/blockandfocus.sock`

4. **Socket permissions:** The production socket is owned by root. Run:
   ```bash
   sudo chmod 666 /var/run/blockandfocus.sock
   ```

### Problem: Sites Not Being Blocked

**Symptom:** You added a domain in the UI but can still access it in the browser.

**Cause 1: Using dev daemon but testing with browser**
- Dev daemon runs on port 5454
- Browser uses system DNS (port 53)
- They're completely separate!

**Solution:** Either:
- Test with `dig @127.0.0.1 -p 5454 domain.com` instead of browser
- Or use production mode with `just install-dev && just set-dns`

**Cause 2: Dev and production configs are different**

There are TWO config files:
- Dev: `./config.toml` in project root
- Production: `/Library/Application Support/BlockAndFocus/config.toml`

If you add domains via the dev UI, they go to the dev config. But if your browser is using the production daemon, it reads the production config.

**Solution:**
```bash
# Copy dev config to production
sudo cp ./config.toml "/Library/Application Support/BlockAndFocus/config.toml"
just reload-daemon
```

**Cause 3: DNS caching**

Browsers and macOS cache DNS results. Even after adding a block, cached results are used.

**Solution:**
```bash
# Flush macOS DNS cache
sudo dscacheutil -flushcache
sudo killall -HUP mDNSResponder

# Also clear browser cache or wait ~60 seconds
```

### Problem: No Internet Access

**Symptom:** Can't load any websites.

**Cause:** System DNS is set to 127.0.0.1 but daemon isn't running.

**Immediate Fix:**
```bash
sudo networksetup -setdnsservers Wi-Fi Empty
sudo dscacheutil -flushcache
```

Then investigate why daemon isn't running:
```bash
# Check if it's running
pgrep -f blockandfocus-daemon

# Check launchd status
sudo launchctl list | grep blockandfocus

# Check daemon logs
sudo cat /var/log/blockandfocus-daemon.log
```

### Problem: Bypass Not Working

**Symptom:** Completed the quiz successfully but sites are still blocked.

**Cause:** Browser DNS caching.

The bypass sets `bypass_until` to a future timestamp. The daemon stops returning 0.0.0.0 for blocked domains. BUT the browser/OS has cached the 0.0.0.0 response.

**Solution:**
```bash
sudo dscacheutil -flushcache
sudo killall -HUP mDNSResponder
```

Or wait ~60 seconds (the TTL of blocked responses).

---

# Part 2: Concepts Explained

This section explains the fundamental technologies used by BlockAndFocus.

## 2.1 DNS Fundamentals

### What is DNS?

DNS (Domain Name System) is like the phone book of the internet. It translates human-readable domain names (like `facebook.com`) into IP addresses (like `157.240.5.35`) that computers use to communicate.

### What Happens When You Type "facebook.com"

```
┌──────────┐          ┌──────────────┐          ┌──────────────┐
│  Browser │──────────│  DNS Server  │──────────│   Facebook   │
│          │  Step 1  │              │  Step 3  │   Server     │
│          │  "What's │              │  Return  │              │
│          │  the IP  │              │  IP:     │              │
│          │  for     │              │157.240.5 │              │
│          │facebook? │              │  .35"    │              │
└──────────┘          └──────────────┘          └──────────────┘
     │                       │                        │
     │                       │                        │
     │◄──────────────────────┘                        │
     │  Step 2: "157.240.5.35"                        │
     │                                                │
     │                                                │
     └────────────────────────────────────────────────┘
              Step 4: Browser connects to 157.240.5.35
```

1. **Browser asks:** "What's the IP address for facebook.com?"
2. **DNS Server responds:** "It's 157.240.5.35"
3. **Browser connects** to 157.240.5.35 and loads the page

### How BlockAndFocus Blocks Sites

We become the DNS server! When your Mac asks "what's facebook.com?", we reply with `0.0.0.0` instead of the real IP.

```
┌──────────┐          ┌──────────────────────┐
│  Browser │──────────│  BlockAndFocus       │
│          │  "What's │  Daemon              │
│          │  the IP  │                      │
│          │  for     │  "facebook.com is    │
│          │ facebook │   in blocklist...    │
│          │  .com?"  │   return 0.0.0.0"    │
└──────────┘          └──────────────────────┘
     │                          │
     │◄─────────────────────────┘
     │  "0.0.0.0"
     │
     │  Browser tries to connect to 0.0.0.0
     │  Connection fails - site "blocked"!
```

When the browser tries to connect to 0.0.0.0, it fails. The site appears unreachable.

### Why DNS Blocking Works

- **System-wide:** ALL applications use DNS, not just browsers
- **Simple:** One place to block, affects everything
- **Fast:** DNS queries are tiny (a few bytes)

### Limitations of DNS Blocking

1. **DNS-over-HTTPS (DoH):** Some apps/browsers use encrypted DNS that bypasses local DNS servers. Chrome, Firefox, etc. may have this enabled.

2. **Cached Results:** If the browser already knows the IP (from cache), it won't ask DNS again.

3. **Direct IP Access:** If someone knows the IP address, they can skip DNS entirely.

4. **VPNs:** VPN traffic may use the VPN's DNS server.

### The `dig` Command

`dig` is your best friend for testing DNS. It queries a DNS server and shows the response.

```bash
# Basic query (uses system DNS)
dig facebook.com

# Query specific server
dig @127.0.0.1 facebook.com

# Query specific port
dig @127.0.0.1 -p 5454 facebook.com

# Short output (just the IP)
dig @127.0.0.1 -p 5454 facebook.com +short
```

Example output:
```bash
$ dig @127.0.0.1 -p 5454 facebook.com +short
0.0.0.0      # Blocked!

$ dig @127.0.0.1 -p 5454 google.com +short
142.250.200.142    # Allowed - real IP
```

## 2.2 Ports and Sockets

### What is a Port?

Think of an IP address as a building's street address. A **port** is like an apartment number within that building.

- IP address `127.0.0.1` = "This computer" (localhost)
- Port `53` = DNS service
- Port `80` = HTTP web traffic
- Port `443` = HTTPS web traffic

So `127.0.0.1:53` means "the DNS service on this computer."

### Why Port 53 Requires Root

On Unix/macOS, ports below 1024 are "privileged" - only root can use them. This is a security feature from the old days.

- Port 53 (DNS) = privileged, needs root
- Port 5454 (our dev port) = unprivileged, any user can use

That's why:
- **Dev mode** uses port 5454 (no sudo needed)
- **Production mode** uses port 53 (requires root)

### What is a Socket?

A socket is an endpoint for communication. There are two types we use:

**1. Network Sockets (TCP/UDP)**
```
Browser ──UDP──> 127.0.0.1:53 ──> BlockAndFocus Daemon
                 (network socket)
```
Used for DNS queries. The daemon listens on a UDP socket.

**2. Unix Domain Sockets**
```
Tauri App ──────> /tmp/blockandfocus-dev.sock ──────> Daemon
                  (file on disk, not network)
```
Used for IPC (Inter-Process Communication). Faster than network sockets because they don't go through the network stack.

### Unix Domain Sockets Explained

A Unix domain socket looks like a file:
```bash
$ ls -la /tmp/blockandfocus-dev.sock
srwxr-xr-x  1 niccolo  wheel  0 Nov 30 21:11 /tmp/blockandfocus-dev.sock
```

The `s` at the start means "socket" (not a regular file).

**Why use them?**
- Faster than TCP/IP (no network overhead)
- More secure (file permissions control access)
- Perfect for local IPC

**Our sockets:**
| Mode | Socket Path | Purpose |
|------|-------------|---------|
| Dev | `/tmp/blockandfocus-dev.sock` | UI ↔ Daemon communication |
| Prod | `/var/run/blockandfocus.sock` | UI ↔ Daemon communication |

## 2.3 launchd (macOS Service Manager)

### What is a Service/Daemon?

A **daemon** is a program that runs in the background, usually starting at boot and running until shutdown. Examples:
- DNS server
- Web server
- Database server

On macOS, **launchd** manages daemons. It's like systemd on Linux.

### Why We Use launchd

1. **Auto-start:** Daemon starts automatically at boot
2. **Keep-alive:** If daemon crashes, launchd restarts it
3. **Port 53 access:** launchd can grant privileged port access

### The plist File

launchd uses `.plist` files (XML format) to configure services.

Our file: `/Library/LaunchDaemons/com.blockandfocus.daemon.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Unique identifier for this service -->
    <key>Label</key>
    <string>com.blockandfocus.daemon</string>

    <!-- Path to the executable -->
    <key>ProgramArguments</key>
    <array>
        <string>/Library/PrivilegedHelperTools/blockandfocus-daemon</string>
    </array>

    <!-- Start immediately when loaded -->
    <key>RunAtLoad</key>
    <true/>

    <!-- Restart if it crashes -->
    <key>KeepAlive</key>
    <true/>

    <!-- Where to write logs -->
    <key>StandardOutPath</key>
    <string>/Library/Logs/BlockAndFocus/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/Library/Logs/BlockAndFocus/daemon.log</string>
</dict>
</plist>
```

### launchd Commands

```bash
# Load a service (start it)
sudo launchctl load /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# Newer way to load (bootstrap)
sudo launchctl bootstrap system /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# Unload a service (stop it)
sudo launchctl unload /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# Newer way to unload (bootout)
sudo launchctl bootout system /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# List all services (find ours)
sudo launchctl list | grep blockandfocus
```

### Why Killing Doesn't Work

When you run `sudo kill -9 <pid>`, launchd sees the daemon died and immediately restarts it (because of `KeepAlive`). That's why you need to use `launchctl bootout` instead.

## 2.4 Dev vs Production Mode

### Why Two Modes?

**Development Mode:**
- Fast iteration (no rebuild + reinstall)
- No root needed
- Doesn't affect your actual internet
- Hot-reload for UI changes

**Production Mode:**
- Actually blocks sites system-wide
- Survives reboots
- Runs as system service

### What Changes Between Modes

| Aspect | Dev Mode | Production Mode |
|--------|----------|-----------------|
| Environment | `BLOCKANDFOCUS_DEV=1` | Not set |
| DNS Port | 5454 | 53 |
| Config File | `./config.toml` | `/Library/Application Support/BlockAndFocus/config.toml` |
| IPC Socket | `/tmp/blockandfocus-dev.sock` | `/var/run/blockandfocus.sock` |
| Root Required | No | Yes |
| Affects Browser | No (use `dig` to test) | Yes |
| Started By | Manual (`just daemon-dev`) | launchd (automatic) |

### The Config File Problem

This caused us a lot of debugging pain! Here's the scenario:

1. You run `just daemon-dev` → Daemon reads `./config.toml`
2. You run `just app-dev` → UI connects to dev daemon
3. You add "x.com" to blocklist via UI → Written to `./config.toml`
4. You test in browser → Browser uses system DNS (port 53)
5. System DNS goes to production daemon → Reads `/Library/Application Support/BlockAndFocus/config.toml`
6. x.com is NOT in production config → Site loads!

**Key Lesson:** When testing with the browser, you MUST use production mode. Dev mode is only for testing with `dig`.

### Port 53 vs 5454

```
                     ┌─────────────────────────────────────┐
                     │            Your Mac                 │
                     │                                     │
                     │   ┌─────────────────────────────┐   │
 dig @127.0.0.1 -p   │   │                             │   │
     5454 ──────────────>│     Dev Daemon              │   │
                     │   │     (port 5454)             │   │
                     │   │     reads ./config.toml     │   │
                     │   └─────────────────────────────┘   │
                     │                                     │
                     │   ┌─────────────────────────────┐   │
 Browser/dig (no     │   │                             │   │
    port) ──────────────>│     Production Daemon       │   │
                     │   │     (port 53)               │   │
                     │   │     reads /Library/...      │   │
                     │   └─────────────────────────────┘   │
                     │                                     │
                     └─────────────────────────────────────┘
```

---

# Part 3: Architecture Deep Dive

## 3.1 System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           User Space                                 │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                    BlockAndFocus.app (Tauri)                   │ │
│  │                                                                 │ │
│  │   ┌─────────────────┐           ┌────────────────────────────┐ │ │
│  │   │                 │           │                            │ │ │
│  │   │   UI (Svelte)   │◄─────────►│     Rust Backend           │ │ │
│  │   │                 │  Tauri    │   (ipc_client.rs, lib.rs)  │ │ │
│  │   │   - App.svelte  │  Bridge   │                            │ │ │
│  │   │   - Components  │           │   - Tauri commands         │ │ │
│  │   │                 │           │   - IPC client             │ │ │
│  │   └─────────────────┘           └────────────────────────────┘ │ │
│  │                                            │                    │ │
│  └────────────────────────────────────────────│────────────────────┘ │
│                                               │                      │
│                                    Unix Domain Socket                │
│                          /var/run/blockandfocus.sock (prod)          │
│                          /tmp/blockandfocus-dev.sock (dev)           │
│                                               │                      │
├───────────────────────────────────────────────│──────────────────────┤
│                           Root Space          │                      │
│                                               │                      │
│  ┌────────────────────────────────────────────│────────────────────┐ │
│  │              blockandfocus-daemon          │                    │ │
│  │              (launchd managed)             ▼                    │ │
│  │                                                                 │ │
│  │   ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐    │ │
│  │   │              │  │              │  │                   │    │ │
│  │   │  DNS Server  │  │   Blocker    │  │    IPC Server     │    │ │
│  │   │ (server.rs)  │  │ (blocker.rs) │  │   (server.rs)     │    │ │
│  │   │              │  │              │  │                   │    │ │
│  │   │ UDP :53/:5454│  │  Domain list │  │ Unix socket       │    │ │
│  │   │              │  │  + matching  │  │ JSON protocol     │    │ │
│  │   └──────────────┘  └──────────────┘  └───────────────────┘    │ │
│  │          │                 │                    │               │ │
│  │          │                 │                    │               │ │
│  │   ┌──────▼──────┐  ┌───────▼──────┐  ┌─────────▼─────────┐    │ │
│  │   │             │  │              │  │                   │    │ │
│  │   │  Upstream   │  │   Schedule   │  │   Quiz Engine     │    │ │
│  │   │ (upstream.rs│  │  (engine.rs) │  │  (generator.rs,   │    │ │
│  │   │             │  │              │  │   validator.rs)   │    │ │
│  │   │ Cloudflare  │  │  Time-based  │  │                   │    │ │
│  │   │  1.1.1.1    │  │  rules       │  │  Math quiz for    │    │ │
│  │   │             │  │              │  │  bypass requests  │    │ │
│  │   └─────────────┘  └──────────────┘  └───────────────────┘    │ │
│  │                                                                 │ │
│  │   ┌─────────────────────────────────────────────────────────┐  │ │
│  │   │                    Config Manager                        │  │ │
│  │   │                    (loader.rs)                           │  │ │
│  │   │                                                          │  │ │
│  │   │    Reads/writes: /Library/.../config.toml (prod)         │  │ │
│  │   │                  ./config.toml (dev)                     │  │ │
│  │   └─────────────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Three Processes

1. **blockandfocus-daemon** (Rust)
   - Runs as root (production) or regular user (dev)
   - Handles DNS queries
   - Manages blocklist and schedule
   - Provides IPC interface

2. **BlockAndFocus.app backend** (Rust via Tauri)
   - Runs as your user
   - Connects to daemon via IPC
   - Provides commands for the UI

3. **BlockAndFocus.app frontend** (Svelte/TypeScript)
   - Runs in a WebView
   - Communicates with backend via Tauri bridge
   - Renders the UI

## 3.2 The Daemon

The daemon is the core of BlockAndFocus. It's defined in `daemon/src/`.

### Components

**AppState** (`main.rs:24-31`)
```rust
pub struct AppState {
    pub config: ConfigManager,      // Loads/saves config
    pub schedule: ScheduleEngine,   // Time-based blocking rules
    pub quiz: QuizEngine,           // Generates/validates quizzes
    pub blocker: DomainBlocker,     // Decides what to block
    pub stats: Stats,               // Query counts
    pub bypass_until: Option<i64>,  // When bypass expires
}
```

**DNS Server** (`dns/server.rs`)
- Binds to UDP port (53 or 5454)
- Receives DNS queries
- Checks with Blocker if domain should be blocked
- Either returns 0.0.0.0 or forwards to upstream

**Domain Blocker** (`dns/blocker.rs`)
- Holds list of blocked domains
- Supports exact match (`facebook.com`) and subdomain match (`www.facebook.com`)
- Can be updated at runtime via IPC

**Upstream Resolver** (`dns/upstream.rs`)
- Forwards allowed queries to real DNS (Cloudflare 1.1.1.1)
- IMPORTANT: Cannot use system DNS config (we ARE the system DNS!)

**IPC Server** (`ipc/server.rs`)
- Unix domain socket server
- JSON-based protocol
- Handles commands from UI: GetStatus, AddDomain, RequestBypass, etc.

**Quiz Engine** (`quiz/`)
- Generates arithmetic problems
- Validates answers with timing checks
- Prevents automation (minimum solve time)

**Schedule Engine** (`schedule/engine.rs`)
- Evaluates time-based rules
- "Block during work hours" etc.

### Main Loop

```rust
// Simplified from main.rs
#[tokio::main]
async fn main() {
    // Load config
    let config = ConfigManager::load(is_dev)?;

    // Create shared state
    let state = Arc::new(RwLock::new(AppState::new(config)));

    // Start DNS server (background task)
    tokio::spawn(DnsServer::run(state.clone()));

    // Start IPC server (background task)
    tokio::spawn(IpcServer::run(state.clone()));

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await;
}
```

## 3.3 The Tauri App

Tauri is a framework for building desktop apps with web technologies. It has two parts:

### Rust Backend (`app/src/`)

**Entry Point** (`main.rs`)
```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_blocklist,
            add_domain,
            remove_domain,
            // ... more commands
        ])
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Tauri Commands** (`lib.rs`)

These are Rust functions that can be called from JavaScript:

```rust
#[tauri::command]
async fn get_status() -> Result<Status, String> {
    let client = IpcClient::connect().await?;
    let response = client.send(Command::GetStatus).await?;
    // ... parse response
}
```

**IPC Client** (`ipc_client.rs`)
```rust
pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    pub async fn connect() -> Result<Self> {
        let socket_path = if is_dev() {
            "/tmp/blockandfocus-dev.sock"
        } else {
            "/var/run/blockandfocus.sock"
        };
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self { stream })
    }

    pub async fn send(&mut self, cmd: Command) -> Result<Response> {
        let json = serde_json::to_string(&cmd)?;
        self.stream.write_all(json.as_bytes()).await?;
        // ... read response
    }
}
```

### Web Frontend (`ui/src/`)

The UI is built with Svelte 5 and communicates with the Rust backend via Tauri's JavaScript API.

```typescript
// In a Svelte component
import { invoke } from '@tauri-apps/api/core';

async function addDomain(domain: string) {
    await invoke('add_domain', { domain });
    // Refresh the list
    blocklist = await invoke('get_blocklist');
}
```

## 3.4 Request Flows

### Flow 1: DNS Query for Blocked Domain

```
┌──────────┐     DNS Query      ┌──────────────┐
│  Browser │ ──────────────────>│   Daemon     │
│          │  "facebook.com?"   │              │
└──────────┘                    │  1. Receive  │
     │                          │     query    │
     │                          │              │
     │                          │  2. Check    │
     │                          │     blocker  │
     │                          │     ✓ BLOCK  │
     │                          │              │
     │                          │  3. Return   │
     │      DNS Response        │     0.0.0.0  │
     │ <────────────────────────│              │
     │      "0.0.0.0"           └──────────────┘
     │
     │  4. Browser tries to
     │     connect to 0.0.0.0
     │
     ▼
  Connection fails!
  Site "blocked"
```

**Code path:**
1. `dns/server.rs:handle_query()` receives UDP packet
2. Parses DNS message, extracts domain name
3. Calls `state.blocker.should_block("facebook.com")`
4. `blocker.rs:should_block()` checks domain list
5. Returns true → `create_blocked_response()` returns 0.0.0.0
6. Sends UDP response back

### Flow 2: DNS Query for Allowed Domain

```
┌──────────┐     DNS Query      ┌──────────────┐     Forward      ┌──────────────┐
│  Browser │ ──────────────────>│   Daemon     │ ────────────────>│  Cloudflare  │
│          │  "google.com?"     │              │  "google.com?"   │    1.1.1.1   │
└──────────┘                    │  1. Check    │                  │              │
     │                          │     blocker  │                  │  2. Lookup   │
     │                          │     ✗ ALLOW  │                  │              │
     │                          │              │ <────────────────│              │
     │                          │  3. Receive  │  "142.250.x.x"   └──────────────┘
     │      DNS Response        │     response │
     │ <────────────────────────│              │
     │   "142.250.200.142"      └──────────────┘
     │
     ▼
  Browser connects to
  142.250.200.142
```

**Code path:**
1. `dns/server.rs:handle_query()` receives query
2. `blocker.should_block("google.com")` returns false
3. `upstream.resolve("google.com")` forwards to Cloudflare
4. Returns real IP to browser

### Flow 3: Adding a Domain via UI

```
┌─────────────┐                  ┌──────────────┐                  ┌──────────────┐
│   UI        │    invoke()      │  Tauri       │   IPC Socket     │   Daemon     │
│  (Svelte)   │ ────────────────>│  Backend     │ ────────────────>│              │
│             │  add_domain      │  (Rust)      │  AddDomain       │  1. Receive  │
│  User types │  "x.com"         │              │  "x.com"         │     command  │
│  "x.com"    │                  │              │                  │              │
│  clicks Add │                  │              │                  │  2. Add to   │
└─────────────┘                  │              │                  │     config   │
      │                          │              │                  │              │
      │                          │              │ <────────────────│  3. Update   │
      │                          │              │    Success       │     blocker  │
      │ <────────────────────────│              │                  │              │
      │         Success          └──────────────┘                  │  4. Save     │
      │                                                            │     config   │
      │                                                            │     file     │
      ▼                                                            └──────────────┘
  UI refreshes
  blocklist
```

**Code path:**
1. `App.svelte` → `invoke('add_domain', { domain: 'x.com' })`
2. `app/src/lib.rs:add_domain()` Tauri command
3. `IpcClient::connect()` → connects to socket
4. Sends `Command::AddDomain { domain: "x.com".into() }`
5. `daemon/src/ipc/server.rs:handle_command()` receives
6. `config.add_domain("x.com")` → updates config + saves file
7. `blocker.update_domains(...)` → updates in-memory list
8. Returns `Response::Success`

### Flow 4: Bypass Quiz Flow

```
┌─────────────┐                  ┌──────────────┐                  ┌──────────────┐
│   UI        │                  │  Tauri       │                  │   Daemon     │
│             │ ────────────────>│  Backend     │ ────────────────>│              │
│  User       │  RequestBypass   │              │  RequestBypass   │  1. Generate │
│  clicks     │  (15 minutes)    │              │                  │     quiz     │
│  "Bypass"   │                  │              │ <────────────────│              │
│             │ <────────────────│              │  QuizChallenge   │  2. Store    │
│             │  Quiz questions  │              │  - questions     │     pending  │
│             │                  │              │  - challenge_id  │              │
│  User sees  │                  │              │  - expires_at    └──────────────┘
│  questions: │                  │              │
│  "23+45=?"  │                  │              │
│  "9×6=?"    │                  │              │
│  "41-39=?"  │                  │              │
│             │                  │              │
│  User types │                  │              │
│  answers:   │                  │              │
│  68, 54, 2  │                  │              │
│             │ ────────────────>│              │ ────────────────>│              │
│  (waits 4+  │  SubmitQuiz      │              │  SubmitQuiz      │  3. Validate │
│   seconds)  │  challenge_id    │              │                  │     - correct│
│             │  answers         │              │                  │       answers│
│             │                  │              │                  │     - timing │
│             │                  │              │ <────────────────│              │
│             │ <────────────────│              │    Success       │  4. Set      │
│             │    Success       │              │                  │     bypass   │
│             │                  │              │                  │     until    │
└─────────────┘                  └──────────────┘                  └──────────────┘

Now daemon's is_blocking_active() returns false until bypass expires!
```

---

# Part 4: Codebase Walkthrough

## 4.1 Directory Structure

```
BlockAndFocus/
├── Cargo.toml              # Workspace definition
├── justfile                # Build commands (like Makefile)
├── config.toml             # DEV config file
├── DEVELOPER_MANUAL.md     # This file!
│
├── daemon/                 # DNS daemon (Rust)
│   ├── Cargo.toml         # Daemon dependencies
│   └── src/
│       ├── main.rs        # Entry point, AppState
│       ├── dns/
│       │   ├── mod.rs     # Module exports
│       │   ├── server.rs  # UDP DNS server
│       │   ├── blocker.rs # Domain matching
│       │   └── upstream.rs# Forward to Cloudflare
│       ├── ipc/
│       │   ├── mod.rs
│       │   └── server.rs  # Unix socket server
│       ├── config/
│       │   ├── mod.rs
│       │   └── loader.rs  # TOML config loading
│       ├── schedule/
│       │   ├── mod.rs
│       │   └── engine.rs  # Time-based rules
│       └── quiz/
│           ├── mod.rs
│           ├── generator.rs # Create questions
│           └── validator.rs # Check answers
│
├── app/                    # Tauri app (Rust backend)
│   ├── Cargo.toml
│   ├── tauri.conf.json    # Tauri config
│   ├── capabilities/      # Tauri v2 permissions
│   └── src/
│       ├── main.rs        # Tauri entry point
│       ├── lib.rs         # Tauri commands
│       └── ipc_client.rs  # Socket client
│
├── ui/                     # Web frontend (Svelte)
│   ├── package.json       # npm dependencies
│   ├── vite.config.ts     # Build config
│   ├── index.html         # HTML entry
│   └── src/
│       ├── main.ts        # JS entry point
│       ├── App.svelte     # Root component
│       ├── app.css        # Styles
│       └── lib/           # Components
│
├── shared/                 # Shared types (Rust)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs         # IPC protocol, Config types
│
└── installer/              # Installation files
    ├── com.blockandfocus.daemon.plist  # launchd config
    └── scripts/
        └── ...
```

### Files You'll Modify Most Often

| Task | File(s) |
|------|---------|
| Add new IPC command | `shared/src/lib.rs`, `daemon/src/ipc/server.rs`, `app/src/lib.rs` |
| Change blocking logic | `daemon/src/dns/blocker.rs` |
| Modify quiz | `daemon/src/quiz/generator.rs` |
| Change UI | `ui/src/App.svelte`, `ui/src/lib/` |
| Add config option | `shared/src/lib.rs`, `daemon/src/config/loader.rs` |

## 4.2 Daemon Code

### `daemon/src/main.rs`

This is the entry point. Key parts:

```rust
// The shared state for the whole daemon
pub struct AppState {
    pub config: ConfigManager,
    pub schedule: ScheduleEngine,
    pub quiz: QuizEngine,
    pub blocker: DomainBlocker,
    pub stats: Stats,
    pub bypass_until: Option<i64>,
}

impl AppState {
    // Check if we should be blocking right now
    pub fn is_blocking_active(&self) -> bool {
        // Check if blocking is enabled
        if !self.config.get().blocking.enabled {
            return false;
        }

        // Check if there's an active bypass
        if let Some(bypass_until) = self.bypass_until {
            let now = chrono::Utc::now().timestamp();
            if now < bypass_until {
                return false;  // Bypass active
            }
        }

        // Check schedule
        if self.config.get().schedule.enabled {
            return self.schedule.is_blocking_time();
        }

        true  // Blocking is active
    }
}
```

**Why `Arc<RwLock<AppState>>`?**

- `Arc` = "Atomically Reference Counted" - allows multiple owners
- `RwLock` = Read-Write Lock - allows multiple readers OR one writer
- Together they enable safe concurrent access from DNS server and IPC server

### `daemon/src/dns/server.rs`

The DNS server using UDP sockets:

```rust
pub struct DnsServer;

impl DnsServer {
    pub async fn run(state: Arc<RwLock<AppState>>) -> Result<()> {
        // Bind to port
        let socket = UdpSocket::bind("127.0.0.1:5454").await?;

        // Main loop - receive queries forever
        loop {
            let (len, src) = socket.recv_from(&mut buf).await?;

            // Handle each query in a separate task
            tokio::spawn(Self::handle_query(
                query_data,
                src,
                socket.clone(),
                state.clone(),
            ));
        }
    }

    async fn handle_query(...) -> Result<()> {
        // Parse DNS query
        let query = Message::from_bytes(&query_data)?;
        let name = query.queries().first()?.name();

        // Should we block?
        let should_block = {
            let state = state.read().await;
            state.is_blocking_active() &&
                state.blocker.should_block(&name.to_string())
        };

        let response = if should_block {
            Self::create_blocked_response(&query)  // 0.0.0.0
        } else {
            upstream.resolve(name).await?  // Real IP
        };

        socket.send_to(&response.to_bytes()?, src).await?;
    }
}
```

### `daemon/src/dns/blocker.rs`

Domain matching logic:

```rust
pub struct DomainBlocker {
    blocked_domains: Vec<String>,
}

impl DomainBlocker {
    pub fn should_block(&self, query_domain: &str) -> bool {
        let normalized = normalize_domain(query_domain);

        for blocked in &self.blocked_domains {
            // Exact match: "facebook.com" == "facebook.com"
            if normalized == *blocked {
                return true;
            }

            // Subdomain match: "www.facebook.com" ends with ".facebook.com"
            if normalized.ends_with(&format!(".{}", blocked)) {
                return true;
            }
        }

        false
    }

    // Called when domains are added/removed via IPC
    pub fn update_domains(&mut self, domains: Vec<String>) {
        self.blocked_domains = domains
            .into_iter()
            .map(|d| normalize_domain(&d))
            .collect();
    }
}

fn normalize_domain(domain: &str) -> String {
    domain
        .to_lowercase()
        .trim()
        .trim_end_matches('.')
        .to_string()
}
```

### `daemon/src/dns/upstream.rs`

Forwarding to real DNS:

```rust
pub struct UpstreamResolver {
    resolver: TokioResolver,
}

impl UpstreamResolver {
    pub fn new() -> Result<Self> {
        // IMPORTANT: Use explicit Cloudflare config!
        // We CANNOT use system DNS because WE ARE the system DNS!
        let config = ResolverConfig::cloudflare();

        let resolver = TokioResolver::builder_with_config(config, ...)
            .build();

        Ok(Self { resolver })
    }

    pub async fn resolve(&self, name: &Name, record_type: RecordType) -> Result<Message> {
        let lookup = self.resolver.lookup_ip(name.to_string()).await?;
        // ... build DNS response from lookup
    }
}
```

**Critical Bug We Fixed:** Originally used system DNS config, which caused infinite loop (daemon queried itself).

### `daemon/src/ipc/server.rs`

Unix socket server for UI communication:

```rust
pub struct IpcServer;

impl IpcServer {
    pub async fn run(state: Arc<RwLock<AppState>>) -> Result<()> {
        let socket_path = if is_dev() {
            "/tmp/blockandfocus-dev.sock"
        } else {
            "/var/run/blockandfocus.sock"
        };

        // Remove old socket file
        let _ = std::fs::remove_file(socket_path);

        // Create new listener
        let listener = UnixListener::bind(socket_path)?;

        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(Self::handle_connection(stream, state.clone()));
        }
    }

    async fn handle_command(cmd: Command, state: &Arc<RwLock<AppState>>) -> Response {
        match cmd {
            Command::Ping => Response::Pong,

            Command::GetStatus => {
                let state = state.read().await;
                Response::Status(Status {
                    blocking_active: state.is_blocking_active(),
                    blocked_domains_count: state.blocker.blocked_count(),
                    // ...
                })
            }

            Command::AddDomain { domain } => {
                let mut state = state.write().await;
                state.config.add_domain(domain).await?;
                // Update in-memory blocker too!
                let domains = state.config.blocked_domains();
                state.blocker.update_domains(domains);
                Response::Success
            }

            // ... other commands
        }
    }
}
```

## 4.3 App Code

### `app/src/main.rs`

Tauri entry point:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_blocklist,
            add_domain,
            remove_domain,
            get_schedule,
            update_schedule,
            request_bypass,
            submit_quiz,
            cancel_bypass,
        ])
        .setup(|app| {
            blockandfocus_app::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### `app/src/lib.rs`

Tauri commands (callable from JavaScript):

```rust
use blockandfocus_shared::{Command, Response, Status};

mod ipc_client;
use ipc_client::IpcClient;

// The #[tauri::command] macro makes this callable from JS
#[tauri::command]
pub async fn get_status() -> Result<Status, String> {
    let mut client = IpcClient::connect()
        .await
        .map_err(|e| e.to_string())?;

    let response = client
        .send(Command::GetStatus)
        .await
        .map_err(|e| e.to_string())?;

    match response {
        Response::Status(status) => Ok(status),
        Response::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}

#[tauri::command]
pub async fn add_domain(domain: String) -> Result<(), String> {
    let mut client = IpcClient::connect().await.map_err(|e| e.to_string())?;

    let response = client
        .send(Command::AddDomain { domain })
        .await
        .map_err(|e| e.to_string())?;

    match response {
        Response::Success => Ok(()),
        Response::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}
```

### `app/src/ipc_client.rs`

Connect to daemon:

```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct IpcClient {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl IpcClient {
    pub async fn connect() -> Result<Self> {
        let socket_path = if std::env::var("BLOCKANDFOCUS_DEV").is_ok() {
            "/tmp/blockandfocus-dev.sock"
        } else {
            "/var/run/blockandfocus.sock"
        };

        let stream = UnixStream::connect(socket_path).await?;
        let (read, write) = stream.into_split();

        Ok(Self {
            reader: BufReader::new(read),
            writer: write,
        })
    }

    pub async fn send(&mut self, command: Command) -> Result<Response> {
        // Send JSON + newline
        let json = serde_json::to_string(&command)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        // Read response
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(&line)?;
        Ok(response)
    }
}
```

## 4.4 UI Code

### `ui/src/main.ts`

JavaScript entry point:

```typescript
import './app.css'
import App from './App.svelte'
import { mount } from 'svelte'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
```

### `ui/src/App.svelte`

Main component (Svelte 5 with runes):

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  // Svelte 5 reactive state using $state rune
  let status = $state<Status | null>(null);
  let blocklist = $state<string[]>([]);
  let newDomain = $state('');
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Load initial data
  onMount(async () => {
    await refresh();
    // Poll for status updates every 5 seconds
    setInterval(refresh, 5000);
  });

  async function refresh() {
    try {
      status = await invoke<Status>('get_status');
      blocklist = await invoke<string[]>('get_blocklist');
      error = null;
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  async function addDomain() {
    if (!newDomain.trim()) return;

    try {
      await invoke('add_domain', { domain: newDomain.trim() });
      newDomain = '';
      await refresh();
    } catch (e) {
      error = e as string;
    }
  }

  async function removeDomain(domain: string) {
    try {
      await invoke('remove_domain', { domain });
      await refresh();
    } catch (e) {
      error = e as string;
    }
  }
</script>

<main>
  {#if error}
    <div class="error">
      <h2>Daemon not connected</h2>
      <p>{error}</p>
    </div>
  {:else if loading}
    <p>Loading...</p>
  {:else}
    <section class="status">
      <h2>Status</h2>
      <p>Blocking: {status?.blocking_active ? 'Active' : 'Inactive'}</p>
      <p>Blocked domains: {status?.blocked_domains_count}</p>
    </section>

    <section class="blocklist">
      <h2>Blocked Domains</h2>

      <form onsubmit={e => { e.preventDefault(); addDomain(); }}>
        <input
          bind:value={newDomain}
          placeholder="example.com"
        />
        <button type="submit">Add</button>
      </form>

      <ul>
        {#each blocklist as domain}
          <li>
            {domain}
            <button onclick={() => removeDomain(domain)}>×</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</main>
```

### Key Svelte Concepts

**Reactivity with `$state`:**
```svelte
<script>
  let count = $state(0);  // Reactive variable

  function increment() {
    count++;  // UI automatically updates
  }
</script>

<button onclick={increment}>{count}</button>
```

**Calling Rust from JavaScript:**
```typescript
import { invoke } from '@tauri-apps/api/core';

// Call Tauri command (defined in app/src/lib.rs)
const result = await invoke('command_name', { arg1: 'value' });
```

## 4.5 Shared Types

### `shared/src/lib.rs`

Types used by both daemon and app:

```rust
use serde::{Deserialize, Serialize};

/// IPC Commands sent from the UI to the daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Command {
    Ping,
    GetStatus,
    GetBlocklist,
    AddDomain { domain: String },
    RemoveDomain { domain: String },
    GetSchedule,
    UpdateSchedule { schedule: Schedule },
    RequestBypass { duration_minutes: u32 },
    SubmitQuizAnswers { challenge_id: String, answers: Vec<i32> },
    CancelBypass,
}

/// IPC Responses sent from the daemon to the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Response {
    Pong,
    Status(Status),
    Blocklist { domains: Vec<String> },
    Schedule(Schedule),
    QuizChallenge(QuizChallenge),
    Success,
    Error { code: ErrorCode, message: String },
}

/// Current daemon status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub blocking_active: bool,
    pub blocked_domains_count: usize,
    pub queries_blocked: u64,
    pub queries_forwarded: u64,
    pub bypass_until: Option<i64>,
    pub active_schedule_rule: Option<String>,
    pub schedule_enabled: bool,
}

/// Config file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dns: DnsConfig,
    pub blocking: BlockingConfig,
    pub schedule: Schedule,
    pub quiz: QuizConfig,
}

// Socket paths
pub const IPC_SOCKET_PATH: &str = "/var/run/blockandfocus.sock";
pub const IPC_SOCKET_PATH_DEV: &str = "/tmp/blockandfocus-dev.sock";
pub const CONFIG_PATH: &str = "/Library/Application Support/BlockAndFocus/config.toml";
pub const CONFIG_PATH_DEV: &str = "./config.toml";
```

**Why `#[serde(tag = "type", content = "payload")]`?**

This controls how enums are serialized to JSON:

```rust
// Without tag
Command::AddDomain { domain: "x.com".into() }
// Serializes to: {"AddDomain":{"domain":"x.com"}}

// With tag = "type", content = "payload"
Command::AddDomain { domain: "x.com".into() }
// Serializes to: {"type":"AddDomain","payload":{"domain":"x.com"}}
```

The tagged format is easier to parse in JavaScript.

---

# Part 5: Making Changes

## 5.1 Common Modifications

### Adding a New IPC Command

Let's say you want to add `GetStats` command.

**Step 1: Define in `shared/src/lib.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Command {
    // ... existing commands
    GetStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Response {
    // ... existing responses
    Stats(StatsData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsData {
    pub total_queries: u64,
    pub blocked_queries: u64,
    pub uptime_seconds: u64,
}
```

**Step 2: Handle in `daemon/src/ipc/server.rs`**

```rust
async fn handle_command(cmd: Command, state: &Arc<RwLock<AppState>>) -> Response {
    match cmd {
        // ... existing handlers

        Command::GetStats => {
            let state = state.read().await;
            Response::Stats(StatsData {
                total_queries: state.stats.queries_blocked + state.stats.queries_forwarded,
                blocked_queries: state.stats.queries_blocked,
                uptime_seconds: state.get_uptime(),
            })
        }
    }
}
```

**Step 3: Add Tauri command in `app/src/lib.rs`**

```rust
#[tauri::command]
pub async fn get_stats() -> Result<StatsData, String> {
    let mut client = IpcClient::connect().await.map_err(|e| e.to_string())?;
    let response = client.send(Command::GetStats).await.map_err(|e| e.to_string())?;

    match response {
        Response::Stats(data) => Ok(data),
        Response::Error { message, .. } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}
```

**Step 4: Register in `app/src/main.rs`**

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing handlers
    get_stats,
])
```

**Step 5: Use in UI `ui/src/App.svelte`**

```svelte
<script>
  let stats = $state<StatsData | null>(null);

  async function loadStats() {
    stats = await invoke<StatsData>('get_stats');
  }
</script>

<button onclick={loadStats}>Get Stats</button>
{#if stats}
  <p>Total queries: {stats.total_queries}</p>
{/if}
```

### Adding a New Config Option

**Step 1: Add to config struct in `shared/src/lib.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingConfig {
    pub enabled: bool,
    pub domains: Vec<String>,
    pub strict_mode: bool,  // NEW
}

impl Default for BlockingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            domains: vec![/* ... */],
            strict_mode: false,  // NEW
        }
    }
}
```

**Step 2: Use in daemon code**

```rust
// In daemon/src/main.rs or wherever needed
if self.config.get().blocking.strict_mode {
    // Do something different
}
```

**Step 3: Update config file**

```toml
# config.toml
[blocking]
enabled = true
strict_mode = true  # NEW
domains = [...]
```

### Changing the Quiz Difficulty

Edit `daemon/src/quiz/generator.rs`:

```rust
impl QuizEngine {
    pub fn generate_challenge(&self) -> QuizChallenge {
        let mut questions = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..self.config.num_questions {
            // Change operand ranges here
            let a = rng.gen_range(10..100);  // Was 10..99
            let b = rng.gen_range(10..100);

            // Add new operations
            let op = rng.gen_range(0..4);  // Was 0..3
            let (answer, display) = match op {
                0 => (a + b, format!("{} + {} = ?", a, b)),
                1 => (a.max(b) - a.min(b), format!("{} - {} = ?", a.max(b), a.min(b))),
                2 => (a * b, format!("{} × {} = ?", a, b)),
                3 => ((a * b), format!("{} ÷ {} = ?", a * b, b)),  // Division (NEW)
                _ => unreachable!(),
            };

            questions.push(Question { display, answer });
        }

        // ...
    }
}
```

## 5.2 Debugging Tips

### Enable Verbose Logging

```bash
RUST_LOG=debug just daemon-dev
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

You can also filter by module:
```bash
RUST_LOG=blockandfocus_daemon::dns=debug,blockandfocus_daemon::ipc=info just daemon-dev
```

### Test DNS Directly

```bash
# Test blocked domain
dig @127.0.0.1 -p 5454 facebook.com +short
# Expected: 0.0.0.0

# Test allowed domain
dig @127.0.0.1 -p 5454 google.com +short
# Expected: Real IP

# See full DNS response
dig @127.0.0.1 -p 5454 facebook.com

# Test with specific record type
dig @127.0.0.1 -p 5454 facebook.com AAAA +short  # IPv6
```

### Test IPC Directly

```bash
# Ping
echo '{"type":"Ping"}' | nc -U /tmp/blockandfocus-dev.sock

# Get status
echo '{"type":"GetStatus"}' | nc -U /tmp/blockandfocus-dev.sock | jq

# Add domain
echo '{"type":"AddDomain","payload":{"domain":"test.com"}}' | nc -U /tmp/blockandfocus-dev.sock

# Request bypass
echo '{"type":"RequestBypass","payload":{"duration_minutes":15}}' | nc -U /tmp/blockandfocus-dev.sock
```

### Common Debug Scenarios

**"Why isn't my domain being blocked?"**

1. Check if blocking is active:
   ```bash
   echo '{"type":"GetStatus"}' | nc -U /tmp/blockandfocus-dev.sock
   # Look for "blocking_active": true
   ```

2. Check if domain is in blocklist:
   ```bash
   echo '{"type":"GetBlocklist"}' | nc -U /tmp/blockandfocus-dev.sock
   ```

3. Check if you're querying the right daemon:
   ```bash
   # Dev daemon (port 5454)
   dig @127.0.0.1 -p 5454 domain.com +short

   # Production daemon (port 53)
   dig @127.0.0.1 domain.com +short
   ```

4. Check daemon logs for the query:
   ```bash
   # Should see: "Received DNS query" or "Blocking DNS query"
   ```

**"Why is the UI showing 'Daemon not connected'?"**

1. Check if socket exists:
   ```bash
   ls -la /tmp/blockandfocus-dev.sock
   ```

2. Check if daemon is running:
   ```bash
   pgrep -f blockandfocus-daemon
   ```

3. Check if you're running in the right mode:
   - Dev UI needs dev daemon
   - Check `BLOCKANDFOCUS_DEV` environment variable

---

# Part 6: Rust Crash Course

This section explains Rust concepts as they appear in BlockAndFocus.

## Basic Syntax

### Variables and Types

```rust
// Immutable by default
let x = 5;        // Type inferred
let y: i32 = 10;  // Explicit type

// Mutable variable
let mut count = 0;
count += 1;

// Common types
let age: i32 = 25;           // 32-bit integer
let pi: f64 = 3.14;          // 64-bit float
let active: bool = true;     // Boolean
let name: String = String::from("Alice");  // Owned string
let name_ref: &str = "Bob";  // String slice (borrowed)
```

### Functions

```rust
// Function with return type
fn add(a: i32, b: i32) -> i32 {
    a + b  // No semicolon = return value
}

// Function that returns Result
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}
```

### Structs

```rust
// Define a struct
struct Person {
    name: String,
    age: u32,
}

// Implement methods
impl Person {
    // Constructor (convention: named "new")
    fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    // Method that borrows self
    fn greet(&self) -> String {
        format!("Hello, I'm {}", self.name)
    }

    // Method that mutates self
    fn birthday(&mut self) {
        self.age += 1;
    }
}

// Usage
let mut alice = Person::new("Alice".to_string(), 30);
println!("{}", alice.greet());
alice.birthday();
```

### Enums

```rust
// Simple enum
enum Color {
    Red,
    Green,
    Blue,
}

// Enum with data
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}

// Pattern matching
fn handle_message(msg: Message) {
    match msg {
        Message::Quit => println!("Quitting"),
        Message::Move { x, y } => println!("Moving to {}, {}", x, y),
        Message::Write(text) => println!("Writing: {}", text),
    }
}
```

## Ownership and Borrowing

This is Rust's most unique feature. It prevents memory bugs at compile time.

### Ownership Rules

1. Each value has one owner
2. When owner goes out of scope, value is dropped
3. You can transfer ownership (move) or borrow

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;  // s1 is MOVED to s2
    // println!("{}", s1);  // ERROR: s1 is no longer valid

    let s3 = s2.clone();  // Explicit copy
    println!("{} {}", s2, s3);  // Both valid
}
```

### Borrowing

```rust
fn main() {
    let s = String::from("hello");

    // Immutable borrow (&)
    let len = calculate_length(&s);
    println!("Length of '{}' is {}", s, len);  // s still valid

    // Mutable borrow (&mut)
    let mut s2 = String::from("hello");
    change(&mut s2);
    println!("{}", s2);  // "hello world"
}

fn calculate_length(s: &String) -> usize {
    s.len()  // Can read, cannot modify
}

fn change(s: &mut String) {
    s.push_str(" world");  // Can modify
}
```

### In BlockAndFocus

```rust
// AppState is shared between tasks
let state = Arc<RwLock<AppState>>;

// Multiple tasks can read simultaneously
let state_guard = state.read().await;  // Borrows state
let is_active = state_guard.is_blocking_active();

// Only one task can write at a time
let mut state_guard = state.write().await;  // Mutable borrow
state_guard.bypass_until = Some(timestamp);
```

## Result and Error Handling

Rust doesn't have exceptions. Instead, errors are values.

```rust
// Result<T, E> = Ok(T) or Err(E)
fn read_file(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

// Using Result
fn main() {
    // Method 1: match
    match read_file("config.toml") {
        Ok(content) => println!("{}", content),
        Err(e) => println!("Error: {}", e),
    }

    // Method 2: unwrap (panics on error - avoid in production)
    let content = read_file("config.toml").unwrap();

    // Method 3: expect (panics with message)
    let content = read_file("config.toml").expect("Failed to read config");

    // Method 4: ? operator (propagates error)
    let content = read_file("config.toml")?;  // Returns Err if error
}
```

### The `?` Operator

```rust
// Without ?
fn load_config() -> Result<Config, Error> {
    let content = match std::fs::read_to_string("config.toml") {
        Ok(c) => c,
        Err(e) => return Err(e.into()),
    };
    let config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => return Err(e.into()),
    };
    Ok(config)
}

// With ? (much cleaner)
fn load_config() -> Result<Config, Error> {
    let content = std::fs::read_to_string("config.toml")?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
```

## Async/Await

Rust's async is zero-cost abstraction for concurrent programming.

```rust
// Async function
async fn fetch_data(url: &str) -> Result<String, Error> {
    // .await pauses until the future completes
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}

// Using async
#[tokio::main]  // Tokio is the async runtime
async fn main() {
    // Run async function
    let data = fetch_data("https://example.com").await.unwrap();

    // Run multiple concurrently
    let (result1, result2) = tokio::join!(
        fetch_data("https://example1.com"),
        fetch_data("https://example2.com"),
    );
}
```

### Spawning Tasks

```rust
// Spawn a background task
tokio::spawn(async move {
    loop {
        do_something().await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
});

// In BlockAndFocus
tokio::spawn(DnsServer::run(state.clone()));  // DNS server task
tokio::spawn(IpcServer::run(state.clone()));  // IPC server task
```

## Common Patterns in BlockAndFocus

### Arc and RwLock

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

// Arc = Atomic Reference Counting (shared ownership)
// RwLock = Read-Write Lock (multiple readers OR one writer)

let state = Arc::new(RwLock::new(AppState::new()));

// Clone Arc (cheap, just increments counter)
let state_clone = state.clone();

// Read access (multiple can read simultaneously)
let guard = state.read().await;
let value = guard.some_field;
// guard dropped here, lock released

// Write access (exclusive)
let mut guard = state.write().await;
guard.some_field = new_value;
// guard dropped here, lock released
```

### Serde for Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    port: u16,
}

// To JSON
let config = Config { name: "test".into(), port: 8080 };
let json = serde_json::to_string(&config)?;
// {"name":"test","port":8080}

// From JSON
let config: Config = serde_json::from_str(&json)?;

// To TOML
let toml = toml::to_string(&config)?;

// From TOML
let config: Config = toml::from_str(&toml)?;
```

---

# Part 7: TypeScript & Svelte Crash Course

## TypeScript Basics

TypeScript is JavaScript with types.

### Variables and Types

```typescript
// Type inference
let count = 0;  // number
let name = "Alice";  // string

// Explicit types
let age: number = 25;
let active: boolean = true;
let items: string[] = ["a", "b", "c"];

// Objects
let person: { name: string; age: number } = {
    name: "Alice",
    age: 25
};

// Using interfaces
interface Person {
    name: string;
    age: number;
}

let alice: Person = { name: "Alice", age: 25 };
```

### Functions

```typescript
// Typed function
function add(a: number, b: number): number {
    return a + b;
}

// Arrow function
const multiply = (a: number, b: number): number => a * b;

// Optional parameters
function greet(name: string, greeting?: string): string {
    return `${greeting ?? "Hello"}, ${name}`;
}

// Async function
async function fetchData(url: string): Promise<string> {
    const response = await fetch(url);
    return response.text();
}
```

### Generics

```typescript
// Generic function
function first<T>(arr: T[]): T | undefined {
    return arr[0];
}

const num = first([1, 2, 3]);  // number
const str = first(["a", "b"]);  // string

// In Tauri's invoke
const status = await invoke<Status>('get_status');
// TypeScript knows status is of type Status
```

## Svelte 5 Basics

Svelte is a component framework. Code compiles to vanilla JS (no virtual DOM).

### Component Structure

```svelte
<!-- MyComponent.svelte -->
<script lang="ts">
  // TypeScript code here
  let name = $state("World");
</script>

<!-- HTML template -->
<h1>Hello {name}!</h1>

<style>
  /* Scoped CSS */
  h1 { color: blue; }
</style>
```

### Reactivity with $state

```svelte
<script lang="ts">
  // $state creates reactive variable
  let count = $state(0);

  function increment() {
    count++;  // UI automatically updates!
  }
</script>

<button onclick={increment}>
  Clicked {count} times
</button>
```

### Derived State with $derived

```svelte
<script lang="ts">
  let count = $state(0);

  // Automatically recalculated when count changes
  let doubled = $derived(count * 2);
</script>

<p>{count} × 2 = {doubled}</p>
```

### Conditional Rendering

```svelte
<script lang="ts">
  let loggedIn = $state(false);
</script>

{#if loggedIn}
  <p>Welcome back!</p>
{:else}
  <p>Please log in</p>
{/if}
```

### Lists

```svelte
<script lang="ts">
  let items = $state(["Apple", "Banana", "Cherry"]);
</script>

<ul>
  {#each items as item, index}
    <li>{index + 1}. {item}</li>
  {/each}
</ul>
```

### Event Handling

```svelte
<script lang="ts">
  let name = $state("");

  function handleSubmit(e: Event) {
    e.preventDefault();
    console.log("Submitted:", name);
  }
</script>

<form onsubmit={handleSubmit}>
  <input bind:value={name} />
  <button type="submit">Submit</button>
</form>
```

### Calling Tauri from Svelte

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface Status {
    blocking_active: boolean;
    blocked_domains_count: number;
  }

  let status = $state<Status | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      status = await invoke<Status>('get_status');
    } catch (e) {
      error = e as string;
    }
  });
</script>

{#if error}
  <p class="error">{error}</p>
{:else if status}
  <p>Blocking: {status.blocking_active ? "Yes" : "No"}</p>
{:else}
  <p>Loading...</p>
{/if}
```

### Component Props

```svelte
<!-- Child.svelte -->
<script lang="ts">
  // Props are declared with $props()
  let { name, age = 0 } = $props<{ name: string; age?: number }>();
</script>

<p>{name} is {age} years old</p>

<!-- Parent.svelte -->
<script lang="ts">
  import Child from './Child.svelte';
</script>

<Child name="Alice" age={25} />
<Child name="Bob" />  <!-- age defaults to 0 -->
```

---

# Quick Reference

## Key Files by Task

| I want to... | Edit these files |
|--------------|------------------|
| Add a blocked domain category | `config.toml`, maybe `blocker.rs` |
| Change quiz difficulty | `daemon/src/quiz/generator.rs` |
| Add new IPC command | `shared/src/lib.rs`, `daemon/src/ipc/server.rs`, `app/src/lib.rs` |
| Modify blocking logic | `daemon/src/dns/blocker.rs` |
| Change the UI | `ui/src/App.svelte`, `ui/src/lib/*.svelte` |
| Add config option | `shared/src/lib.rs`, `daemon/src/config/loader.rs` |
| Debug daemon | Run with `RUST_LOG=debug` |

## Emergency Commands

```bash
# Restore internet
sudo networksetup -setdnsservers Wi-Fi Empty && sudo dscacheutil -flushcache

# Kill everything
sudo pkill -9 -f blockandfocus-daemon && pkill -f "cargo"

# Stop production daemon
sudo launchctl bootout system /Library/LaunchDaemons/com.blockandfocus.daemon.plist

# Check what's using ports
sudo lsof -i :53
sudo lsof -i :5454
```

## Development Workflow

```bash
# Terminal 1: Start daemon
just daemon-dev

# Terminal 2: Start UI
just app-dev

# Terminal 3: Test
dig @127.0.0.1 -p 5454 facebook.com +short
echo '{"type":"GetStatus"}' | nc -U /tmp/blockandfocus-dev.sock
```

---

*This manual was generated based on real debugging sessions and covers the actual pain points encountered while developing and testing BlockAndFocus.*
