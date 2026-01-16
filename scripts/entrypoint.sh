#!/bin/bash
set -e

# Allows users to specify the UID/GID the container should run as
# This ensures mounted volumes have correct permissions

PUID=${PUID:-1000}
PGID=${PGID:-1000}

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
    chown -R rustatio:rustatio /app /data

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

    echo "Starting rustatio-server with UID=$PUID GID=$PGID"

    # Drop privileges and run the server
    exec gosu rustatio "$@"
else
    # Already running as non-root, just run the command
    echo "Starting rustatio-server as $(whoami)"
    exec "$@"
fi
