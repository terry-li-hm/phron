#!/usr/bin/env bash
set -e

echo "Setting up keychain credentials for comes (phron)..."
echo "These values are used by the launchd jobs which don't have access to 1Password CLI."

read -p "Enter Oura Personal Access Token: " -s oura_token
echo
security add-generic-password -a "$USER" -s "oura-token" -w "$oura_token"

read -p "Enter Telegram Bot Token: " -s bot_token
echo
security add-generic-password -a "$USER" -s "telegram-bot-token" -w "$bot_token"

read -p "Enter Telegram Chat ID: " -s chat_id
echo
security add-generic-password -a "$USER" -s "telegram-chat-id" -w "$chat_id"

read -p "Enter Anthropic API Key: " -s anthropic_key
echo
security add-generic-password -a "$USER" -s "anthropic-api-key" -w "$anthropic_key"

read -p "Enter OpenRouter API Key: " -s openrouter_key
echo
security add-generic-password -a "$USER" -s "openrouter-api-key" -w "$openrouter_key"

echo "Keychain setup complete!"
