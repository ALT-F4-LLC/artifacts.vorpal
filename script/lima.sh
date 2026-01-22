#!/usr/bin/env bash
set -euo pipefail

function deps {
    sudo apt-get update

    sudo apt-get upgrade --yes

    sudo apt-get install --yes \
        bubblewrap \
        build-essential \
        ca-certificates \
        curl \
        jq \
        rsync \
        unzip \
        wget

    if ! command -v docker &> /dev/null; then
        echo "Docker not found. Installing Docker..."

        curl -fsSL https://get.docker.com -o /tmp/get-docker.sh

        sudo sh /tmp/get-docker.sh

        rm /tmp/get-docker.sh

        sudo usermod -aG docker "${USER}"
    fi

    if ! command -v vorpal &> /dev/null; then
        wget "https://raw.githubusercontent.com/ALT-F4-LLC/vorpal/refs/heads/main/script/install.sh" -O /tmp/install-vorpal.sh

        chmod +x /tmp/install-vorpal.sh

        /tmp/install-vorpal.sh --yes

        rm /tmp/install-vorpal.sh
    fi

    # Add AppArmor profile for bubblewrap if AppArmor is being used
    if command -v apparmor_status &> /dev/null && sudo apparmor_status &> /dev/null; then
        echo "AppArmor detected. Adding bubblewrap profile..."

        sudo tee /etc/apparmor.d/bwrap > /dev/null << 'EOF'
abi <abi/4.0>,
include <tunables/global>

profile bwrap /usr/bin/bwrap flags=(unconfined) {
  userns,

  # Site-specific additions and overrides. See local/README for details.
  include if exists <local/bwrap>
}
EOF

        sudo apparmor_parser -r /etc/apparmor.d/bwrap
        echo "AppArmor bubblewrap profile installed and loaded."
    fi
}

function sync {
    deps

    mkdir -p "$HOME/source"

    rsync -aPW \
    --delete \
    --exclude=".git" \
    --exclude="target" \
    "$PWD/." "$HOME/source/."
}

COMMAND="${1:-}"

if [[ -z "$COMMAND" ]]; then
    echo "Usage: $0 <command>"
    echo "Available commands: deps, install, sync"
    exit 1
fi

if [[ "$COMMAND" != "deps" && "$COMMAND" != "install" && "$COMMAND" != "sync" ]]; then
    echo "Invalid command: $COMMAND"
    echo "Available commands: deps, install, sync"
    exit 1
fi

export PATH="$HOME/.vorpal/bin:$PATH"

if [[ "$COMMAND" == "deps" ]]; then
    deps
    exit 0
fi

if [[ "$COMMAND" == "sync" ]]; then
    sync
    exit 0
fi
