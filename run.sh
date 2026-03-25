#!/bin/bash
set -euo pipefail

WORKDIR=/home/kooraseru/Mod-Mail
cd "$WORKDIR"

COMMAND="${1:-start}"

service_status() {
  systemctl --user status discord-bot --no-pager || true
}

stop_service() {
  echo "=== Stopping service ==="
  systemctl --user stop discord-bot || true
  echo "Service stopped."
}

# Trap Ctrl+C to cleanly stop the service
trap 'echo ""; echo "Caught Ctrl+C — stopping service..."; stop_service; exit 0' INT

case "$COMMAND" in

  start)
    mkdir -p .local .logs

    # Extract DISCORD_TOKEN from .actrc
    TOKEN=$(grep -- '-s DISCORD_TOKEN=' .actrc | sed 's/.*-s DISCORD_TOKEN=//')
    if [[ -z "$TOKEN" ]]; then
      echo "ERROR: DISCORD_TOKEN not found in .actrc" >&2
      exit 1
    fi
    echo "DISCORD_TOKEN=$TOKEN" > .local/discord-bot.env
    chmod 600 .local/discord-bot.env

    # Build binary + write service file
    echo "=== Running act workflow ==="
    act workflow_dispatch -j local-deploy -W .github/workflows/local-deploy.yml 2>&1 | tee .logs/recent.log

    # Install and start user-level systemd service
    SERVICE_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SERVICE_DIR"
    cp .local/discord-bot.service "$SERVICE_DIR/discord-bot.service"

    systemctl --user daemon-reload
    systemctl --user enable discord-bot
    systemctl --user restart discord-bot

    echo ""
    echo "=== Service Status ==="
    service_status

    # Append to deploy log
    {
      echo "=== Deployment $(date '+%Y-%m-%d %H:%M:%S') ==="
      echo "--- systemctl status ---"
      service_status
      echo "--- journalctl (last 50 lines) ---"
      journalctl --user -u discord-bot -n 50 --no-pager || true
      echo "=== End of deployment log ==="
    } | tee -a .logs/deploy.log

    echo ""
    echo "Bot is running. Press Ctrl+C to stop."
    # Tail logs so the terminal stays open and Ctrl+C can be caught
    tail -f .logs/discord-bot.log
    ;;

  stop)
    stop_service
    ;;

  pause)
    echo "=== Pausing service (SIGSTOP) ==="
    systemctl --user kill --kill-who=main --signal=SIGSTOP discord-bot
    echo "Service paused. Run '$0 resume' to continue."
    ;;

  resume)
    echo "=== Resuming service (SIGCONT) ==="
    systemctl --user kill --kill-who=main --signal=SIGCONT discord-bot
    echo "Service resumed."
    service_status
    ;;

  status)
    service_status
    ;;

  *)
    echo "Usage: $0 {start|stop|pause|resume|status}"
    exit 1
    ;;
esac