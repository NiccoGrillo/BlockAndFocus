# BlockAndFocus

A DNS-based domain blocker for macOS that helps you stay focused and productive by blocking distracting websites.

## Features

- **DNS-level blocking**: Blocks domains at the DNS level, affecting all applications
- **Schedule-based blocking**: Configure blocking to activate during specific hours (e.g., 9am-5pm on weekdays)
- **Arithmetic quiz bypass**: To temporarily disable blocking, you must solve math problems (friction to prevent impulsive disabling)
- **Menu bar app**: Easy-to-use Tauri-based UI in your menu bar
- **Configurable blocklist**: Add or remove domains easily
- **Lightweight**: Minimal resource usage

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Space                                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              BlockAndFocus.app (Tauri)                     │  │
│  │         Menu bar UI for configuration                      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                    Unix Domain Socket                            │
│                              │                                   │
├──────────────────────────────┼───────────────────────────────────┤
│                        Root Space                                │
│  ┌───────────────────────────┼───────────────────────────────┐  │
│  │         blockandfocus-daemon (launchd)                     │  │
│  │  DNS Server (port 53) + Blocker + IPC + Quiz Engine        │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Requirements

- macOS 12.0 (Monterey) or later
- [Rust](https://rustup.rs) (for building from source)
- [just](https://github.com/casey/just) command runner (optional but recommended)

## Installation

### From Source

1. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and build**:
   ```bash
   git clone https://github.com/yourusername/BlockAndFocus.git
   cd BlockAndFocus
   cargo build --release
   ```

3. **Install the daemon** (requires admin privileges):
   ```bash
   # If you have just installed:
   just install-dev

   # Or manually:
   sudo mkdir -p /Library/Application\ Support/BlockAndFocus
   sudo mkdir -p /Library/Logs/BlockAndFocus
   sudo cp target/release/blockandfocus-daemon /Library/PrivilegedHelperTools/
   sudo cp installer/com.blockandfocus.daemon.plist /Library/LaunchDaemons/
   sudo chmod 755 /Library/PrivilegedHelperTools/blockandfocus-daemon
   sudo chmod 644 /Library/LaunchDaemons/com.blockandfocus.daemon.plist
   sudo launchctl load /Library/LaunchDaemons/com.blockandfocus.daemon.plist
   ```

4. **Configure system DNS**:
   ```bash
   # Set DNS to use BlockAndFocus
   sudo networksetup -setdnsservers Wi-Fi 127.0.0.1
   sudo dscacheutil -flushcache
   ```

## Development

### Quick Start

```bash
# Run daemon in development mode (port 5353, no root needed)
just daemon-dev

# In another terminal, test DNS blocking
dig @127.0.0.1 -p 5353 facebook.com
```

### Available Commands

```bash
just                  # Show all available commands
just daemon-dev       # Run daemon in development mode
just build            # Build release binaries
just test             # Run all tests
just install-dev      # Install daemon for testing (requires sudo)
just uninstall-dev    # Uninstall daemon
just set-dns          # Configure system to use BlockAndFocus
just restore-dns      # Restore system DNS to automatic
```

### Testing DNS Blocking

```bash
# Start the daemon in dev mode
just daemon-dev

# Test blocked domain (should return 0.0.0.0)
dig @127.0.0.1 -p 5353 facebook.com +short
# Expected: 0.0.0.0

# Test allowed domain (should return real IP)
dig @127.0.0.1 -p 5353 google.com +short
# Expected: Real IP address
```

### Testing IPC

```bash
# With daemon running, test IPC commands
just ipc-ping       # Should return Pong
just ipc-status     # Get daemon status
just ipc-blocklist  # Get current blocklist
```

## Configuration

Configuration is stored at `/Library/Application Support/BlockAndFocus/config.toml`:

```toml
[dns]
upstream = ["1.1.1.1", "8.8.8.8"]
listen_address = "127.0.0.1"
listen_port = 53

[blocking]
enabled = true
domains = [
    "facebook.com",
    "twitter.com",
    "instagram.com",
    "reddit.com",
    "tiktok.com",
]

[schedule]
enabled = true

[[schedule.rules]]
name = "Work Hours"
days = ["mon", "tue", "wed", "thu", "fri"]
start_time = "09:00"
end_time = "17:00"

[quiz]
num_questions = 3
min_operand = 10
max_operand = 99
timeout_seconds = 60
min_solve_seconds = 3
```

## Uninstallation

```bash
# If you have just installed:
just uninstall-dev
just restore-dns

# Or manually:
sudo launchctl unload /Library/LaunchDaemons/com.blockandfocus.daemon.plist
sudo rm /Library/LaunchDaemons/com.blockandfocus.daemon.plist
sudo rm /Library/PrivilegedHelperTools/blockandfocus-daemon
sudo rm -rf /Library/Application\ Support/BlockAndFocus
sudo networksetup -setdnsservers Wi-Fi Empty
sudo dscacheutil -flushcache
```

## How It Works

1. **DNS Interception**: The daemon listens on port 53 (localhost only) and intercepts all DNS queries from your system.

2. **Domain Blocking**: When a blocked domain is queried, the daemon returns `0.0.0.0` instead of the real IP address, effectively blocking access.

3. **Schedule Enforcement**: Blocking can be configured to only activate during certain hours/days.

4. **Quiz Bypass**: To temporarily disable blocking, you must solve arithmetic problems. This creates friction that prevents impulsive unblocking.

## Security Considerations

- The daemon runs as root (required for port 53) but only accepts connections from localhost
- Configuration files are owned by root with restricted permissions
- Quiz validation happens server-side in the daemon (cannot be bypassed by UI manipulation)
- The daemon auto-restarts via launchd if killed

## Troubleshooting

### DNS not working after installation

1. Make sure the daemon is running: `pgrep blockandfocus-daemon`
2. Check daemon logs: `cat /Library/Logs/BlockAndFocus/daemon.log`
3. Verify DNS is set: `scutil --dns | head -20`
4. Flush DNS cache: `sudo dscacheutil -flushcache`

### Daemon won't start

1. Check for port conflicts: `sudo lsof -i :53`
2. Check launchd status: `sudo launchctl list | grep blockandfocus`
3. Try running manually: `sudo /Library/PrivilegedHelperTools/blockandfocus-daemon`

### Want to bypass blocking

Use the quiz system through the menu bar app, or temporarily disable by running:
```bash
sudo launchctl unload /Library/LaunchDaemons/com.blockandfocus.daemon.plist
sudo networksetup -setdnsservers Wi-Fi Empty
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
