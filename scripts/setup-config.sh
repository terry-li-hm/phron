#!/usr/bin/env bash
set -e

CONFIG_DIR="$HOME/.config/comes"
CONFIG_FILE="$CONFIG_DIR/config.toml"
TEMPLATE_FILE="$(dirname "$0")/../config.toml.example"

echo "Setting up comes configuration..."

mkdir -p "$CONFIG_DIR"

if [ -f "$CONFIG_FILE" ]; then
    echo "Config file already exists at $CONFIG_FILE"
else
    cp "$TEMPLATE_FILE" "$CONFIG_FILE"
    echo "Copied config template to $CONFIG_FILE"
    echo "Please edit $CONFIG_FILE with your settings."
fi
