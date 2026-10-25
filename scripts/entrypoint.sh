#!/bin/bash
set -e

# Allows users to specify the UID/GID the container should run as
# This ensures mounted volumes have correct permissions

PUID=${PUID:-1000}
PGID=${PGID:-1000}

DAEMON_ENABLED=${DAEMON_ENABLED:-true}
DAEMON_SCRIPT=${DAEMON_SCRIPT:-/app/rustatio_daemon.py}

# Lance rustatio_daemon en arrière-plan avec relance automatique.
# $1 (optionnel) : utilisateur sous lequel exécuter le daemon.
start_daemon() {
    case "$DAEMON_ENABLED" in
        true|1|yes) ;;
        *) echo "[Daemon] Désactivé (DAEMON_ENABLED=$DAEMON_ENABLED)"; return 0 ;;
    esac

    if [ ! -f "$DAEMON_SCRIPT" ]; then
        echo "[Daemon] $DAEMON_SCRIPT introuvable, daemon non démarré"
        return 0
    fi

    (
        trap 'exit 0' TERM INT
        while true; do
            echo "[Daemon] Lancement de rustatio_daemon..."
            code=0
            if [ -n "$1" ]; then
                gosu "$1" python3 "$DAEMON_SCRIPT" --daemon-mode || code=$?
            else
                python3 "$DAEMON_SCRIPT" --daemon-mode || code=$?
            fi
            echo "[Daemon] Arrêt inattendu (code $code). Redémarrage dans 3 secondes..."
            sleep 3 & wait $!
        done
    ) &
}

if [ ! -f "$DATA_DIR/rules.txt" ]; then
    if [ -f /app/rules.default.txt ]; then
        cp /app/rules.default.txt "$DATA_DIR/rules.txt"
        echo "=> Fichier rules.txt par défaut copié dans $DATA_DIR"
    fi
fi
if [ ! -f "$DATA_DIR/state.json" ]; then
    if [ -f /app/state.default.json ]; then
        cp /app/state.default.json "$DATA_DIR/state.json"
        echo "=> Fichier state.json par défaut copié dans $DATA_DIR"
    fi
fi

# Only modify user/group if running as root
if [ "$(id -u)" = "0" ]; then
    # Update rustatio group GID if different
    if [ "$(id -g rustatio)" != "$PGID" ]; then
        groupmod -o -g "$PGID" rustatio
    fi

    # Update rustatio user UID if different
    if [ "$(id -u rustatio)" != "$PUID" ]; then
        usermod -o -u "$PUID" rustatio
    fi

    # Ensure ownership of app directories
    chown -R rustatio:rustatio /app "$DATA_DIR"

    # Check watch directory permissions if it exists (mounted volume)
    if [ -d "$WATCH_DIR" ]; then
        # Test if we can write to the directory
        if ! su rustatio -s /bin/sh -c "test -w '$WATCH_DIR'" 2>/dev/null; then
            echo "============================================================"
            echo "WARNING: Watch folder '$WATCH_DIR' is not writable!"
            echo ""
            echo "The container user (PUID=$PUID/PGID=$PGID) cannot write to"
            echo "the mounted watch folder. Please fix permissions on the host:"
            echo ""
            echo "  Option 1: Change ownership to match PUID/PGID"
            echo "    sudo chown -R $PUID:$PGID /path/to/your/torrents"
            echo ""
            echo "  Option 2: Make directory world-writable"
            echo "    chmod 777 /path/to/your/torrents"
            echo ""
            echo "  Option 3: Create directory before mounting"
            echo "    mkdir -p /path/to/your/torrents"
            echo "============================================================"
            echo ""
        fi
    fi

    start_daemon rustatio

    echo "Starting rustatio-server with UID=$PUID GID=$PGID"

    # Drop privileges and run the server
    exec gosu rustatio "$@"
else
    # Already running as non-root, just run the command
    start_daemon

    echo "Starting rustatio-server as $(whoami)"
    exec "$@"
fi
