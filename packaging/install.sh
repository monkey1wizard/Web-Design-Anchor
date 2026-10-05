#!/usr/bin/env bash
# WDA installer
# Usage: curl -fsSL https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.sh | bash
#
# SHA-256 verification is mandatory. Cosign verification is best effort when
# cosign or its signature files are unavailable, but a completed failed
# verification aborts the installation.

set -euo pipefail

REPO="monkey1wizard/web-design-anchor"
REPO_CANONICAL="monkey1wizard/web-design-anchor"
INSTALL_DIR="${HOME}/.local/bin"
TMP_DIR=""

die() { echo "wda-install: error: $*" >&2; exit 1; }
warn() { echo "wda-install: warning: $*" >&2; }
info() { echo "wda-install: $*"; }

cleanup() {
    [ -n "${TMP_DIR}" ] && rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Darwin) os="darwin" ;;
        Linux) os="linux" ;;
        *) die "Unsupported OS: $(uname -s). Supported: Linux, macOS."
           ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) os="${os}"; arch="x64" ;;
        aarch64|arm64) os="${os}"; arch="arm64" ;;
        *) die "Unsupported architecture: $(uname -m). Supported: x86_64, aarch64."
           ;;
    esac

    echo "${os} ${arch}"
}

fetch_latest_version() {
    local tag
    tag=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
    [ -n "${tag}" ] || die "Failed to fetch latest release version from GitHub API."
    echo "${tag}"
}

sha256_file() {
    local file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${file}" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "${file}" | awk '{print $1}'
    else
        die "No SHA-256 tool found (sha256sum or shasum required)."
    fi
}

main() {
    local os arch platform version archive_name base_url expected actual
    platform=$(detect_platform)
    read -r os arch <<< "${platform}"
    info "Detected platform: ${os}-${arch}"

    version="${WDA_VERSION:-$(fetch_latest_version)}"
    info "Installing WDA ${version}"

    archive_name="wda-${version}-${os}-${arch}.tar.gz"
    base_url="${WDA_RELEASE_BASE_URL:-https://github.com/${REPO}/releases/download/${version}}"
    base_url="${base_url%/}"
    TMP_DIR=$(mktemp -d)

    info "Downloading ${archive_name}..."
    curl -fsSL --retry 3 --retry-delay 2 \
        "${base_url}/${archive_name}" \
        -o "${TMP_DIR}/${archive_name}" \
        || die "Download failed: ${base_url}/${archive_name}"

    info "Downloading checksums.txt..."
    curl -fsSL --retry 3 --retry-delay 2 \
        "${base_url}/checksums.txt" \
        -o "${TMP_DIR}/checksums.txt" \
        || die "Download failed: ${base_url}/checksums.txt"

    info "Verifying SHA-256 integrity..."
    expected=$(awk -v name="${archive_name}" '$2 == name || $2 == "*" name { print $1; exit }' \
        "${TMP_DIR}/checksums.txt")
    [ -n "${expected}" ] \
        || die "Checksum entry not found for '${archive_name}' in checksums.txt."

    actual=$(sha256_file "${TMP_DIR}/${archive_name}")
    if [ "${expected}" != "${actual}" ]; then
        die "SHA-256 mismatch - download may be corrupted or tampered.
  Expected: ${expected}
  Actual:   ${actual}
Aborting installation."
    fi
    info "SHA-256 OK (${actual:0:16}...)"

    if command -v cosign >/dev/null 2>&1; then
        info "cosign found - downloading signature files..."
        if curl -fsSL --retry 2 "${base_url}/checksums.txt.sig" -o "${TMP_DIR}/checksums.txt.sig" 2>/dev/null \
        && curl -fsSL --retry 2 "${base_url}/checksums.txt.pem" -o "${TMP_DIR}/checksums.txt.pem" 2>/dev/null; then
            if cosign verify-blob \
                --signature "${TMP_DIR}/checksums.txt.sig" \
                --certificate "${TMP_DIR}/checksums.txt.pem" \
                --certificate-identity-regexp "(?i)https://github.com/${REPO_CANONICAL}/\.github/workflows/release\.yml@.*" \
                --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
                "${TMP_DIR}/checksums.txt"; then
                info "cosign signature verified."
            else
                die "cosign signature verification FAILED - checksums.txt does not match a trusted signature. Aborting installation."
            fi
        else
            warn "Could not download cosign signature files - skipping (SHA-256 passed)."
        fi
    else
        warn "cosign not found in PATH - skipping cosign verification (SHA-256 passed)."
    fi

    info "Extracting archive..."
    mkdir -p "${TMP_DIR}/pkg"
    tar -xzf "${TMP_DIR}/${archive_name}" -C "${TMP_DIR}/pkg"
    [ -f "${TMP_DIR}/pkg/wda" ] || die "Binary 'wda' not found in archive."

    mkdir -p "${INSTALL_DIR}"
    install -m 755 "${TMP_DIR}/pkg/wda" "${INSTALL_DIR}/wda"

    echo ""
    echo "WDA installed successfully."
    echo "  Location: ${INSTALL_DIR}/wda"
    echo ""
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo "Add ${INSTALL_DIR} to your PATH by adding this line to your shell profile:"
            echo '  export PATH="${HOME}/.local/bin:${PATH}"'
            echo ""
            ;;
    esac
    echo "WDA requires deno and git on your PATH."
    echo "The Skill ships at the root of the release archive."
}

main "$@"
