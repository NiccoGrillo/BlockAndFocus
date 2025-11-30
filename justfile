# BlockAndFocus Build System
# Run `just` to see available commands

set dotenv-load := true

# Default target triple
target := `rustc -Vv | grep host | cut -f2 -d' '`

# Default recipe - show help
default:
    @just --list

# ============ DEVELOPMENT ============

# Run daemon in development mode (port 5353, no root needed)
daemon-dev:
    @echo "Starting daemon in development mode..."
    BLOCKANDFOCUS_DEV=1 RUST_LOG=debug cargo run --package blockandfocus-daemon

# Run daemon with specific port
daemon-port port:
    BLOCKANDFOCUS_DEV=1 BLOCKANDFOCUS_PORT={{port}} RUST_LOG=debug cargo run --package blockandfocus-daemon

# ============ BUILDING ============

# Build all packages (daemon only, app requires 'just app-build')
build:
    cargo build --release --package blockandfocus-daemon --package blockandfocus-shared

# Build daemon only
build-daemon:
    cargo build --release --package blockandfocus-daemon

# Build with debug info
build-debug:
    cargo build

# ============ TESTING ============

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Test DNS blocking (requires daemon running on port 5353)
test-dns domain="facebook.com":
    @echo "Testing DNS resolution for {{domain}}..."
    dig @127.0.0.1 -p 5353 {{domain}} +short

# Test DNS blocking against production port (requires sudo)
test-dns-prod domain="facebook.com":
    @echo "Testing DNS resolution for {{domain}} on port 53..."
    dig @127.0.0.1 {{domain}} +short

# ============ INSTALLATION ============

# Install daemon for development testing (requires sudo)
install-dev: build-daemon
    @echo "Installing daemon for development..."
    sudo mkdir -p /Library/Application\ Support/BlockAndFocus
    sudo cp target/release/blockandfocus-daemon /Library/PrivilegedHelperTools/
    sudo cp installer/com.blockandfocus.daemon.plist /Library/LaunchDaemons/
    sudo chmod 755 /Library/PrivilegedHelperTools/blockandfocus-daemon
    sudo chmod 644 /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    sudo launchctl load /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    @echo "Daemon installed and started!"

# Uninstall daemon
uninstall-dev:
    @echo "Uninstalling daemon..."
    -sudo launchctl unload /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    -sudo rm /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    -sudo rm /Library/PrivilegedHelperTools/blockandfocus-daemon
    @echo "Daemon uninstalled!"

# Reload daemon (after rebuilding)
reload-daemon: build-daemon
    @echo "Reloading daemon..."
    -sudo launchctl unload /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    sudo cp target/release/blockandfocus-daemon /Library/PrivilegedHelperTools/
    sudo launchctl load /Library/LaunchDaemons/com.blockandfocus.daemon.plist
    @echo "Daemon reloaded!"

# Set system DNS to use BlockAndFocus
set-dns:
    @echo "Setting system DNS to 127.0.0.1..."
    sudo networksetup -setdnsservers Wi-Fi 127.0.0.1
    sudo dscacheutil -flushcache
    @echo "System DNS configured!"

# Restore system DNS to automatic
restore-dns:
    @echo "Restoring system DNS to automatic..."
    sudo networksetup -setdnsservers Wi-Fi Empty
    sudo dscacheutil -flushcache
    @echo "System DNS restored!"

# ============ IPC TESTING ============

# Send a ping command to the daemon
ipc-ping:
    @echo '{"type":"Ping"}' | nc -U /tmp/blockandfocus-dev.sock

# Get daemon status
ipc-status:
    @echo '{"type":"GetStatus"}' | nc -U /tmp/blockandfocus-dev.sock

# Get blocklist
ipc-blocklist:
    @echo '{"type":"GetBlocklist"}' | nc -U /tmp/blockandfocus-dev.sock

# Add domain to blocklist
ipc-add domain:
    @echo '{"type":"AddDomain","payload":{"domain":"{{domain}}"}}' | nc -U /tmp/blockandfocus-dev.sock

# Remove domain from blocklist
ipc-remove domain:
    @echo '{"type":"RemoveDomain","payload":{"domain":"{{domain}}"}}' | nc -U /tmp/blockandfocus-dev.sock

# ============ TAURI APP ============

# Run Tauri app in development mode
app-dev:
    @echo "Starting Tauri app in development mode..."
    @echo "Make sure the daemon is running first with: just daemon-dev"
    BLOCKANDFOCUS_DEV=1 cargo tauri dev

# Build Tauri app for production
app-build:
    cd ui && npm run build
    cargo tauri build

# Build UI only
ui-build:
    cd ui && npm run build

# Install UI dependencies
ui-install:
    cd ui && npm install

# ============ UTILITIES ============

# Check if daemon is running
check:
    @pgrep -f blockandfocus-daemon && echo "Daemon is running" || echo "Daemon is not running"

# View daemon logs
logs:
    @tail -f /Library/Logs/BlockAndFocus/daemon.log 2>/dev/null || echo "No logs found (daemon may be running in dev mode)"

# Clean build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt --all

# Run clippy lints
lint:
    cargo clippy --workspace -- -D warnings

# ============ SETUP ============

# Initial project setup
setup:
    @echo "Setting up BlockAndFocus development environment..."
    @command -v rustc >/dev/null || (echo "Please install Rust first: https://rustup.rs" && exit 1)
    @command -v just >/dev/null || (echo "Please install just: cargo install just" && exit 1)
    cargo build
    @echo ""
    @echo "Setup complete! Run 'just daemon-dev' to start the daemon in development mode."
    @echo "Then in another terminal, test with: just test-dns facebook.com"
