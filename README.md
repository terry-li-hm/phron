# phron (comes)

Personal AI life coach CLI: health monitoring via Oura Ring, morning brief synthesis, overnight research, and proactive nudges via Due app and Telegram.

## Commands
- `comes brief`: Morning brief synthesis
- `comes health`: On-demand health check
- `comes nudge`: Proactive alert check (for LaunchAgents)
- `comes overnight`: Overnight research runner
- `comes status`: Status dashboard

## Installation
```bash
cargo install --path .
```

## Configuration
Run the setup script:
```bash
./scripts/setup-config.sh
```
Edit `~/.config/comes/config.toml` to your liking.

## Environment Variables
The CLI relies on the following environment variables (which should be injected by your shell profile or keychain for LaunchAgents):
- `OURA_TOKEN`
- `ANTHROPIC_API_KEY`
- `OPENROUTER_API_KEY`
- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

You can use the keychain script to set up keychain items if preferred.

## LaunchAgents
Templates are provided in `launchd/`.
Copy them to `~/Library/LaunchAgents/` and load them:
```bash
cp launchd/*.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.phron.nudge.plist
launchctl load ~/Library/LaunchAgents/com.phron.overnight.plist
```
