#!/bin/bash
# BlockAndFocus Post-Installation Script
# This script is run after the package is installed.

set -e

DAEMON_PATH="/Library/PrivilegedHelperTools/blockandfocus-daemon"
PLIST_PATH="/Library/LaunchDaemons/com.blockandfocus.daemon.plist"
CONFIG_DIR="/Library/Application Support/BlockAndFocus"
LOG_DIR="/Library/Logs/BlockAndFocus"

echo "BlockAndFocus: Running post-installation..."

# Create directories
mkdir -p "$CONFIG_DIR"
mkdir -p "$LOG_DIR"

# Set ownership and permissions
chown root:wheel "$DAEMON_PATH"
chmod 755 "$DAEMON_PATH"

chown root:wheel "$PLIST_PATH"
chmod 644 "$PLIST_PATH"

chown -R root:wheel "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"

chown -R root:wheel "$LOG_DIR"
chmod 755 "$LOG_DIR"

# Create default config if it doesn't exist
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
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
EOF
    chown root:wheel "$CONFIG_DIR/config.toml"
    chmod 600 "$CONFIG_DIR/config.toml"
    echo "BlockAndFocus: Created default configuration"
fi

# Load the daemon
echo "BlockAndFocus: Loading daemon..."
launchctl load "$PLIST_PATH" || true

# Configure DNS
echo "BlockAndFocus: Configuring system DNS..."

# Get the primary network service (usually Wi-Fi or Ethernet)
PRIMARY_SERVICE=$(networksetup -listnetworkserviceorder | grep -A1 "Hardware Port" | grep -v "Hardware Port" | head -1 | sed 's/^.*: //' | sed 's/)$//')

if [ -n "$PRIMARY_SERVICE" ]; then
    # Save current DNS settings
    CURRENT_DNS=$(networksetup -getdnsservers "$PRIMARY_SERVICE" 2>/dev/null || echo "")
    echo "$CURRENT_DNS" > "$CONFIG_DIR/.original_dns"

    # Set DNS to localhost
    networksetup -setdnsservers "$PRIMARY_SERVICE" 127.0.0.1
    echo "BlockAndFocus: Set DNS for $PRIMARY_SERVICE to 127.0.0.1"
fi

# Also try Wi-Fi specifically
networksetup -setdnsservers "Wi-Fi" 127.0.0.1 2>/dev/null || true

# Flush DNS cache
dscacheutil -flushcache
killall -HUP mDNSResponder 2>/dev/null || true

echo "BlockAndFocus: Installation complete!"
echo ""
echo "The daemon is now running and blocking configured domains."
echo "To manage blocked domains, use the BlockAndFocus menu bar app."
echo ""
echo "To uninstall, run: /Library/Application Support/BlockAndFocus/uninstall.sh"

exit 0
