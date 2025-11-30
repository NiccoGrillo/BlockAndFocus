#!/bin/bash
# BlockAndFocus Uninstallation Script

set -e

DAEMON_PATH="/Library/PrivilegedHelperTools/blockandfocus-daemon"
PLIST_PATH="/Library/LaunchDaemons/com.blockandfocus.daemon.plist"
CONFIG_DIR="/Library/Application Support/BlockAndFocus"
LOG_DIR="/Library/Logs/BlockAndFocus"
APP_PATH="/Applications/BlockAndFocus.app"

echo "BlockAndFocus: Starting uninstallation..."

# Stop and unload the daemon
echo "BlockAndFocus: Stopping daemon..."
launchctl unload "$PLIST_PATH" 2>/dev/null || true

# Restore original DNS settings
if [ -f "$CONFIG_DIR/.original_dns" ]; then
    ORIGINAL_DNS=$(cat "$CONFIG_DIR/.original_dns")

    # Get primary service
    PRIMARY_SERVICE=$(networksetup -listnetworkserviceorder | grep -A1 "Hardware Port" | grep -v "Hardware Port" | head -1 | sed 's/^.*: //' | sed 's/)$//')

    if [ -n "$PRIMARY_SERVICE" ]; then
        if [ "$ORIGINAL_DNS" = "There aren't any DNS Servers set on" ] || [ -z "$ORIGINAL_DNS" ]; then
            networksetup -setdnsservers "$PRIMARY_SERVICE" Empty
            echo "BlockAndFocus: Restored DNS to automatic for $PRIMARY_SERVICE"
        else
            networksetup -setdnsservers "$PRIMARY_SERVICE" $ORIGINAL_DNS
            echo "BlockAndFocus: Restored original DNS for $PRIMARY_SERVICE"
        fi
    fi
fi

# Also restore Wi-Fi DNS
networksetup -setdnsservers "Wi-Fi" Empty 2>/dev/null || true

# Flush DNS cache
dscacheutil -flushcache
killall -HUP mDNSResponder 2>/dev/null || true

# Remove files
echo "BlockAndFocus: Removing files..."
rm -f "$PLIST_PATH"
rm -f "$DAEMON_PATH"

# Ask about config removal
echo ""
read -p "Remove configuration and logs? (y/N) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$CONFIG_DIR"
    rm -rf "$LOG_DIR"
    echo "BlockAndFocus: Configuration and logs removed"
else
    echo "BlockAndFocus: Configuration preserved at $CONFIG_DIR"
fi

# Remove app if exists
if [ -d "$APP_PATH" ]; then
    rm -rf "$APP_PATH"
    echo "BlockAndFocus: Application removed"
fi

echo ""
echo "BlockAndFocus: Uninstallation complete!"
echo "Your DNS settings have been restored."

exit 0
