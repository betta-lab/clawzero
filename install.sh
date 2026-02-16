#!/bin/sh
# clawzero installer
# Usage: curl -fsSL https://raw.githubusercontent.com/betta-lab/clawzero/main/install.sh | sh
set -eu

REPO="betta-lab/clawzero"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '\033[1;34m%s\033[0m\n' "$*"; }
error() { printf '\033[1;31merror: %s\033[0m\n' "$*" >&2; exit 1; }

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os_part="unknown-linux-gnu" ;;
        Darwin) os_part="apple-darwin" ;;
        *)      error "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64)         arch_part="x86_64" ;;
        aarch64|arm64)  arch_part="aarch64" ;;
        *)              error "Unsupported architecture: $arch" ;;
    esac

    echo "${arch_part}-${os_part}"
}

get_latest_tag() {
    url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
        tag=$(curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        tag=$(wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
    else
        error "curl or wget is required"
    fi
    [ -z "$tag" ] && error "Failed to fetch latest release tag"
    echo "$tag"
}

download() {
    url="$1"; dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$dest" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    fi
}

verify_checksum() {
    archive="$1"; checksums="$2"
    filename="$(basename "$archive")"

    expected="$(grep "$filename" "$checksums" | awk '{print $1}')"
    [ -z "$expected" ] && error "Checksum not found for $filename"

    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$archive" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
    else
        info "Warning: sha256sum/shasum not found, skipping checksum verification"
        return 0
    fi

    [ "$actual" = "$expected" ] || error "Checksum mismatch: expected $expected, got $actual"
}

main() {
    target="$(detect_target)"
    info "Detected target: $target"

    tag="$(get_latest_tag)"
    info "Latest release: $tag"

    base_url="https://github.com/${REPO}/releases/download/${tag}"
    archive_name="clawzero-${tag}-${target}.tar.gz"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading $archive_name ..."
    download "${base_url}/${archive_name}" "${tmpdir}/${archive_name}"
    download "${base_url}/SHA256SUMS.txt" "${tmpdir}/SHA256SUMS.txt"

    info "Verifying checksum ..."
    verify_checksum "${tmpdir}/${archive_name}" "${tmpdir}/SHA256SUMS.txt"

    info "Extracting ..."
    tar xzf "${tmpdir}/${archive_name}" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    cp "${tmpdir}/clawzero-${tag}-${target}/clawzero" "$INSTALL_DIR/clawzero"
    chmod +x "$INSTALL_DIR/clawzero"

    info "Installed clawzero to $INSTALL_DIR/clawzero"

    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            info ""
            info "Add $INSTALL_DIR to your PATH:"
            info "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac
}

main
