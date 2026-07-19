#!/usr/bin/env bash

# Standard installation/update script for a Linux server.
#
# Assumptions:
#   1. The Git repository has been cloned to a neutral location.
#   2. cargo is installed.
#   3. npx is installed.
#   4. jq is installed.
#   5. rsync is installed.
#   6. systemd is used.
#   7. A reverse proxy is used.

set -Eeuo pipefail

SERVICE_USER="party-hub"
CONFIG_DIR="/etc/party-hub"
DATA_DIR="/var/lib/party-hub"
ASSET_DIR="${DATA_DIR}/assets"
BINARY_PATH="/usr/local/bin/party-hub"
SERVICE_PATH="/etc/systemd/system/party-hub.service"

PUBLIC_KEY_PATH="${CONFIG_DIR}/public_vapid_key.pem"
PRIVATE_KEY_PATH="${CONFIG_DIR}/private_vapid_key.pem"
CONFIG_PATH="${CONFIG_DIR}/config.env"

# Ensure the script is being run from the repository root.
for required_path in \
    "./Cargo.toml" \
    "./prod-config.env" \
    "./party-hub-default.service" \
    "./pages" \
    "./static"
do
    if [[ ! -e "${required_path}" ]]; then
        echo "Error: required path not found: ${required_path}" >&2
        echo "Run this script from the repository root." >&2
        exit 1
    fi
done

# 1. Compile binary
echo "Building party-hub..."
cargo build --release

if [[ ! -x "./target/release/party-hub" ]]; then
    echo "Error: compiled binary was not found." >&2
    exit 1
fi

# 2. Create the system user if it does not already exist
if id "${SERVICE_USER}" >/dev/null 2>&1; then
    echo "System user ${SERVICE_USER} already exists; keeping it."
else
    echo "Creating system user ${SERVICE_USER}..."
    sudo useradd \
        --system \
        --no-create-home \
        --shell /usr/sbin/nologin \
        "${SERVICE_USER}"
fi

# 3. Create directories with correct ownership and permissions
echo "Creating application directories..."
sudo install -d -o root -g root -m 755 "${CONFIG_DIR}"
sudo install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${DATA_DIR}"
sudo install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${ASSET_DIR}"
sudo install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${ASSET_DIR}/pages"
sudo install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${ASSET_DIR}/static"

# 4. Generate VAPID keys only when neither key exists
if [[ -e "${PUBLIC_KEY_PATH}" && -e "${PRIVATE_KEY_PATH}" ]]; then
    echo "VAPID keys already exist; keeping them."

elif [[ -e "${PUBLIC_KEY_PATH}" || -e "${PRIVATE_KEY_PATH}" ]]; then
    echo "Error: only one VAPID key exists." >&2
    echo "Existing key files have not been changed." >&2
    echo "Restore the missing matching key before running this script again." >&2
    exit 1

else
    echo "Generating VAPID keys..."

    key_file="$(mktemp)"
    trap 'rm -f "${key_file:-}"' EXIT
    chmod 600 "${key_file}"

    npx web-push generate-vapid-keys --json > "${key_file}"

    # Confirm that both values exist before installing either file.
    jq -e '
        (.publicKey | type == "string" and length > 0) and
        (.privateKey | type == "string" and length > 0)
    ' "${key_file}" >/dev/null

    jq -r '.publicKey' "${key_file}" |
        sudo install \
            -o root \
            -g "${SERVICE_USER}" \
            -m 640 \
            /dev/stdin \
            "${PUBLIC_KEY_PATH}"

    jq -r '.privateKey' "${key_file}" |
        sudo install \
            -o root \
            -g "${SERVICE_USER}" \
            -m 640 \
            /dev/stdin \
            "${PRIVATE_KEY_PATH}"

    rm -f "${key_file}"
    trap - EXIT

    echo "VAPID keys installed."
fi

# 5. Install or update the binary
echo "Installing application binary..."
sudo install \
    -o root \
    -g root \
    -m 755 \
    "./target/release/party-hub" \
    "${BINARY_PATH}"

# 6. Install or update application assets
#
# --delete removes obsolete files only within the managed pages/static
# directories. It does not affect configuration, keys, or other application
# data.
echo "Installing application assets..."
sudo rsync \
    --archive \
    --delete \
    --chown="${SERVICE_USER}:${SERVICE_USER}" \
    --chmod=D750,F640 \
    "./pages/" \
    "${ASSET_DIR}/pages/"

sudo rsync \
    --archive \
    --delete \
    --chown="${SERVICE_USER}:${SERVICE_USER}" \
    --chmod=D750,F640 \
    "./static/" \
    "${ASSET_DIR}/static/"

# 7. Install the default configuration only if none exists
if [[ -e "${CONFIG_PATH}" ]]; then
    echo "Configuration already exists; keeping ${CONFIG_PATH}."
else
    echo "Installing default configuration..."
    sudo install \
        -o root \
        -g "${SERVICE_USER}" \
        -m 640 \
        "./prod-config.env" \
        "${CONFIG_PATH}"
fi

# 8. Install the systemd service only if none exists
service_installed=false

if [[ -e "${SERVICE_PATH}" ]]; then
    echo "Systemd service already exists; keeping ${SERVICE_PATH}."
else
    echo "Installing systemd service..."
    sudo install \
        -o root \
        -g root \
        -m 644 \
        "./party-hub-default.service" \
        "${SERVICE_PATH}"

    service_installed=true
fi

# Reload systemd only when the service file was installed.
if [[ "${service_installed}" == true ]]; then
    echo "Reloading systemd configuration..."
    sudo systemctl daemon-reload
fi

cat <<EOF

Installation/update completed.

Next steps:
  1. Check ${CONFIG_PATH}.
  2. Ensure your reverse proxy is configured correctly.
  3. Enable and start the service:
       sudo systemctl enable --now party-hub
  4. For an existing installation, restart it to use the new binary/assets:
       sudo systemctl restart party-hub
  5. Create your first author account, if needed:
       ./createAuthor.sh "Your Name"
EOF
