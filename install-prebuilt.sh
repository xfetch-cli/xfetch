#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Prebuilt installer — downloads a precompiled binary from GitHub Releases.
# Fast: no Rust toolchain, no compilation. Requires only curl (and tar/unzip).
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install-prebuilt.sh | bash
#        bash install-prebuilt.sh --version 1.0.1
#        bash install-prebuilt.sh --prefix /usr/local --yes

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO="xfetch-cli/xfetch"
REPO_URL="https://github.com/${REPO}"
REPO_API="https://api.github.com/repos/${REPO}"
REPO_RAW="https://raw.githubusercontent.com/${REPO}/main"

# Keyless (OIDC) signing identity used for release artifacts.
# Releases from SIGNED_MIN_VERSION onward are expected to publish a
# SHA256SUMS.bundle; older releases are unsigned and only SHA-256 verified.
SIGNING_WORKFLOW="https://github.com/${REPO}/.github/workflows/release.yml"
SIGNING_OIDC_ISSUER="https://token.actions.githubusercontent.com"
SIGNED_MIN_VERSION="1.0.1"

PROJECT="xfetch"
PROJECT_DESC="cross-platform system information fetcher (prebuilt binary)"

# Default paths (may be overridden by flags)
PREFIX="${PREFIX:-${HOME}/.local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
CONFIG_DIR="${CONFIG_DIR:-${HOME}/.config/${PROJECT}}"

# Behavior flags
FLAG_VERBOSE=1
FLAG_MODIFY_PATH=1
FLAG_YES=0
FLAG_SKIP_CONFIG=0
FLAG_NO_CHECKSUM=0
FLAG_ALLOW_UNSIGNED=0
FLAG_REQUIRE_SIGNATURE=0
VERSION=""          # empty = latest release

# Runtime
TEMP_DIR=""
EXISTING_CONFIG=0
OS_NAME=""
ARCH_NAME=""
TARGET_TRIPLE=""

# ──────────────────────────────────────────────
# Utility functions
# ──────────────────────────────────────────────

log()   { printf '\033[1;34m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
ok()    { printf '\033[1;32m[%s]\033[0m %s\n' "${PROJECT}" "$*"; }
warn()  { printf '\033[1;33m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
error() { printf '\033[1;31m[%s]\033[0m %s\n' "${PROJECT}" "$*" >&2; }
die()   { error "$*"; exit 1; }

has_tty() { [ -t 0 ]; }

cleanup() {
    local exit_code=$?
    if [ -n "${TEMP_DIR}" ] && [ -d "${TEMP_DIR}" ]; then
        rm -rf "${TEMP_DIR}" 2>/dev/null || true
    fi
    if [ "${exit_code}" -ne 0 ]; then
        error "Installation failed (exit code: ${exit_code}). See errors above."
    fi
    exit "${exit_code}"
}
trap cleanup EXIT INT TERM HUP

# ──────────────────────────────────────────────
# OS / Architecture detection
# ──────────────────────────────────────────────

detect_os() {
    case "$(uname -s)" in
        Linux)  echo "linux" ;;
        Darwin) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)      echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             echo "unknown" ;;
    esac
}

# True on musl-based Linux (Alpine, ...), where the glibc build cannot run.
is_musl() {
    if [ -e /lib/ld-musl-x86_64.so.1 ] || [ -e /lib/ld-musl-aarch64.so.1 ]; then
        return 0
    fi
    ldd --version 2>&1 | grep -qi musl
}

# Map detected OS/arch to the rust target triple used in release assets
resolve_target() {
    case "${OS_NAME}-${ARCH_NAME}" in
        linux-x86_64)
            if is_musl; then
                TARGET_TRIPLE="x86_64-unknown-linux-musl"
            else
                TARGET_TRIPLE="x86_64-unknown-linux-gnu"
            fi
            ;;
        linux-aarch64) TARGET_TRIPLE="aarch64-unknown-linux-gnu" ;;
        macos-x86_64)  TARGET_TRIPLE="x86_64-apple-darwin" ;;
        macos-aarch64) TARGET_TRIPLE="aarch64-apple-darwin" ;;
        windows-x86_64) die "On Windows use PowerShell: irm ${REPO_RAW}/install-prebuilt.ps1 | iex" ;;
        *) die "No prebuilt binary for ${OS_NAME}-${ARCH_NAME}. Use install.sh (builds from source)." ;;
    esac
    [ "${FLAG_VERBOSE}" -eq 1 ] && log "Target: ${TARGET_TRIPLE}"
}

detect_shell_rc() {
    local shell_name
    shell_name="$(basename "${SHELL:-${HOME}}" 2>/dev/null || echo "bash")"
    shell_name="${shell_name%%[0-9]*}"
    case "${shell_name}" in
        zsh)
            if [ -n "${ZDOTDIR:-}" ]; then
                echo "${ZDOTDIR}/.zshrc"
            else
                echo "${HOME}/.zshrc"
            fi
            ;;
        bash)
            if [ "${OS_NAME}" = "macos" ]; then
                echo "${HOME}/.bash_profile"
            else
                echo "${HOME}/.bashrc"
            fi
            ;;
        fish) echo "${HOME}/.config/fish/config.fish" ;;
        *)
            for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
                if [ -f "${rc}" ]; then
                    echo "${rc}"
                    return 0
                fi
            done
            echo "${HOME}/.profile"
            ;;
    esac
}

# ──────────────────────────────────────────────
# Argument parsing
# ──────────────────────────────────────────────

usage() {
    cat <<EOF
${PROJECT} — ${PROJECT_DESC}

Usage:
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash
  bash install-prebuilt.sh [options]

Options:
  -h, --help              Show this help message and exit
  -p, --prefix <dir>      Installation prefix (default: \${HOME}/.local)
  -b, --bin-dir <dir>     Binary install directory (default: \${PREFIX}/bin)
  -c, --config-dir <dir>  Config directory (default: \${HOME}/.config/${PROJECT})
  -n, --no-modify-path    Do not modify shell config files to add to PATH
  -y, --yes               Automatic yes to all prompts
  -s, --skip-config       Skip generating the default config
  -q, --quiet             Quiet mode (minimal output)
      --version <ver>     Install a specific version (e.g. 1.0.1); default: latest release
      --require-signature Abort when cosign is unavailable or the signature bundle is missing
      --allow-unsigned    Accept a signed release that is missing SHA256SUMS.bundle
                          (SHA-256 only; not recommended)
      --no-checksum       Disable SHA-256 AND signature verification (dangerous)

Environment variables:
  PREFIX                  Same as --prefix
  BIN_DIR                 Same as --bin-dir
  CONFIG_DIR              Same as --config-dir

Verification:
  Prebuilt installs always fetch the release SHA256SUMS and abort when it is
  missing or does not list the archive. From v1.0.1 onward every release is
  also signed: the checksums file is verified with cosign (keyless OIDC), and a
  missing SHA256SUMS.bundle aborts the install unless --allow-unsigned is
  given. If cosign itself is not installed you get a warning and the install
  continues after the SHA-256 check; pass --require-signature to make a missing
  cosign fatal. --no-checksum disables BOTH checks and is dangerous.
  Hashing uses sha256sum, shasum -a 256 or openssl (whichever exists).

Examples:
  # Quick install (latest release, no sudo needed)
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash

  # Install a specific version
  curl -fsSL ${REPO_RAW}/install-prebuilt.sh | bash -s -- --version 1.0.1

  # Non-interactive install
  bash install-prebuilt.sh --yes

  # System-wide install
  bash install-prebuilt.sh --prefix /usr/local --yes

  # Install without PATH modification
  bash install-prebuilt.sh --no-modify-path

Report issues: ${REPO_URL}/issues
EOF
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -n|--no-modify-path) FLAG_MODIFY_PATH=0 ;;
            -y|--yes) FLAG_YES=1 ;;
            -s|--skip-config) FLAG_SKIP_CONFIG=1 ;;
            -q|--quiet) FLAG_VERBOSE=0 ;;
            --no-checksum) FLAG_NO_CHECKSUM=1 ;;
            --allow-unsigned) FLAG_ALLOW_UNSIGNED=1 ;;
            --require-signature) FLAG_REQUIRE_SIGNATURE=1 ;;
            --version)
                shift; VERSION="$1"
                ;;
            -p|--prefix)
                shift; PREFIX="$1"
                [ -z "${BIN_DIR_OVERRIDE:-}" ] && BIN_DIR="${PREFIX}/bin"
                ;;
            -b|--bin-dir)
                shift; BIN_DIR="$1"; BIN_DIR_OVERRIDE=1
                ;;
            -c|--config-dir)
                shift; CONFIG_DIR="$1"
                ;;
            --) shift; break ;;
            -*)
                die "Unknown option: $1. Use --help for usage."
                ;;
            *) break ;;
        esac
        shift
    done
}

# ──────────────────────────────────────────────
# Download and checksum
# ──────────────────────────────────────────────

# A release version must be a dotted numeric version, optionally with a
# prerelease suffix (1.0.1, 1.0.1-rc1, 1.0.1-rc.2). Anything else is rejected
# instead of being silently coerced.
validate_version() {
    printf '%s\n' "${1:-}" \
        | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$'
}

resolve_asset_url() {
    local release_url
    if [ -z "${VERSION}" ]; then
        release_url="${REPO_API}/releases/latest"
    else
        validate_version "${VERSION}" \
            || die "Invalid --version '${VERSION}': expected a dotted numeric release such as 1.0.1."
        release_url="${REPO_API}/releases/tags/v${VERSION}"
    fi

    log "Resolving ${PROJECT} release..."
    local json
    json="$(curl -fsSL --max-time 30 "${release_url}")" || die "Cannot reach ${release_url}"

    # The release tag is the ONLY authoritative version source. It must never be
    # derived from asset filenames: a release-write attacker could otherwise
    # publish "xfetch-0.9.9-..." on a v1.0.1 release and downgrade the
    # signature policy to the pre-1.0.1 "unsigned is fine" branch.
    local tag_name tag_version
    tag_name="$(printf '%s' "${json}" \
        | grep -o '"tag_name": *"[^"]*"' \
        | head -n 1 \
        | sed 's/.*"tag_name": *"//; s/"$//')" || tag_name=""
    [ -n "${tag_name}" ] || die "Release metadata does not include a tag_name. Refusing to install."

    tag_version="${tag_name#v}"
    validate_version "${tag_version}" \
        || die "Release tag '${tag_name}' is not a valid version. Refusing to install."

    if [ -n "${VERSION}" ] && [ "${VERSION}" != "${tag_version}" ]; then
        die "Requested version v${VERSION} does not match release tag ${tag_name}. Refusing to install."
    fi

    # From here on VERSION is the validated tag version.
    VERSION="${tag_version}"

    # Look for the exact asset name produced by the release workflow for this
    # tag and target. Regex matching is deliberately avoided so that a decoy
    # asset cannot be selected.
    local expected_base="${PROJECT}-${VERSION}-${TARGET_TRIPLE}"
    local urls url file
    urls="$(printf '%s' "${json}" \
        | grep -o '"browser_download_url": *"[^"]*"' \
        | sed 's/.*"browser_download_url": *"//; s/"$//')" || urls=""
    [ -n "${urls}" ] || die "Release ${tag_name} has no downloadable assets. Refusing to install."

    ASSET_URL=""
    ASSET_FILE=""
    while IFS= read -r url; do
        [ -n "${url}" ] || continue
        file="${url##*/}"
        if [ "${file}" = "${expected_base}.tar.gz" ] || [ "${file}" = "${expected_base}.zip" ]; then
            ASSET_URL="${url}"
            ASSET_FILE="${file}"
            break
        fi
    done <<< "${urls}"

    if [ -z "${ASSET_URL}" ]; then
        die "Release ${tag_name} has no asset named ${expected_base}.tar.gz or ${expected_base}.zip. Refusing to install an unexpected asset."
    fi

    log "Found: ${ASSET_FILE} (v${VERSION}, tag ${tag_name})"
}

# True when an SHA-256 implementation is available on this system.
# sha256sum is GNU coreutils (Linux), shasum ships with macOS, openssl is the
# portable last resort.
has_hash_tool() {
    command -v sha256sum >/dev/null 2>&1 \
        || command -v shasum >/dev/null 2>&1 \
        || command -v openssl >/dev/null 2>&1
}

# Print the lowercase SHA-256 of a file, or return non-zero when no supported
# hashing tool exists. Never silently degrades: callers must handle failure.
sha256_file() {
    local file="$1"
    local out
    if command -v sha256sum >/dev/null 2>&1; then
        out="$(sha256sum "${file}")" || return 1
        printf '%s\n' "${out%% *}"
    elif command -v shasum >/dev/null 2>&1; then
        out="$(shasum -a 256 "${file}")" || return 1
        printf '%s\n' "${out%% *}"
    elif command -v openssl >/dev/null 2>&1; then
        out="$(openssl dgst -sha256 "${file}")" || return 1
        printf '%s\n' "${out##* }"
    else
        return 1
    fi
}

# version_ge A B -> success when dotted numeric version A >= B.
# Uses awk to stay portable (BSD/macOS sort has no reliable -V).
version_ge() {
    awk -v a="$1" -v b="$2" 'BEGIN {
        na = split(a, A, "."); nb = split(b, B, ".");
        n = (na > nb) ? na : nb;
        for (i = 1; i <= n; i++) {
            x = (i <= na) ? A[i] + 0 : 0;
            y = (i <= nb) ? B[i] + 0 : 0;
            if (x > y) exit 0;
            if (x < y) exit 1;
        }
        exit 0;
    }'
}

# Verify SHA256SUMS with the keyless cosign signature.
#
# Policy:
#   * bundle + cosign      -> signature must verify (a bad signature is fatal);
#   * no cosign            -> warn and continue, unless --require-signature;
#   * missing bundle       -> fatal for releases >= SIGNED_MIN_VERSION unless
#                             --allow-unsigned, warning for older releases.
# This function never depends on a local hashing tool being installed.
verify_signature() {
    local sums_path="$1"
    local bundle_url="${REPO_URL}/releases/download/v${VERSION}/SHA256SUMS.bundle"
    local bundle_path="${TEMP_DIR}/SHA256SUMS.bundle"
    local expects_signature=0

    if version_ge "${VERSION}" "${SIGNED_MIN_VERSION}"; then
        expects_signature=1
    fi

    # Check bundle presence before looking for cosign: the signed-release
    # policy must not become fail-open just because cosign is not installed.
    local bundle_present=0
    if curl -fsSL --max-time 30 "${bundle_url}" -o "${bundle_path}" 2>/dev/null \
        && [ -s "${bundle_path}" ]; then
        bundle_present=1
    fi

    if [ "${bundle_present}" -eq 0 ]; then
        if [ "${FLAG_REQUIRE_SIGNATURE}" -eq 1 ]; then
            die "--require-signature was given but no signature bundle was found for v${VERSION} (${bundle_url}). Refusing to install."
        fi
        if [ "${expects_signature}" -eq 1 ]; then
            if [ "${FLAG_ALLOW_UNSIGNED}" -eq 1 ]; then
                warn "SHA256SUMS.bundle is missing for v${VERSION}; continuing with SHA-256 only because --allow-unsigned was given."
                return 0
            fi
            die "SHA256SUMS.bundle is missing for v${VERSION}, which is expected to be signed. Refusing to install. Re-run with --allow-unsigned to accept SHA-256-only verification (not recommended)."
        fi
        warn "No signature bundle for v${VERSION} (releases before v${SIGNED_MIN_VERSION} were unsigned); SHA-256 only."
        return 0
    fi

    if ! command -v cosign >/dev/null 2>&1; then
        if [ "${FLAG_REQUIRE_SIGNATURE}" -eq 1 ]; then
            die "--require-signature was given but cosign is not installed. Install cosign from https://docs.sigstore.dev/cosign/system_config/installation/ and retry."
        fi
        warn "cosign is not installed — the signature for v${VERSION} will NOT be verified."
        warn "Install cosign, or re-run with --require-signature to make this fatal."
        return 0
    fi

    local identity="${SIGNING_WORKFLOW}@refs/tags/v${VERSION}"
    log "Verifying SHA256SUMS signature with cosign (keyless OIDC)..."

    if cosign verify-blob \
        --bundle "${bundle_path}" \
        --certificate-identity "${identity}" \
        --certificate-oidc-issuer "${SIGNING_OIDC_ISSUER}" \
        "${sums_path}" >/dev/null 2>&1; then
        ok "Signature verified (keyless OIDC: ${SIGNING_WORKFLOW})."
    else
        die "Signature verification failed for SHA256SUMS. Refusing to install. (cosign >= 2.2 with --bundle support is required; upgrade cosign and retry.)"
    fi
}

download_and_verify() {
    TEMP_DIR="$(mktemp -d)"
    log "Downloading ${ASSET_FILE}..."
    curl -fSL --max-time 300 "${ASSET_URL}" -o "${TEMP_DIR}/${ASSET_FILE}" \
        || die "Download failed: ${ASSET_URL}"

    if [ "${FLAG_NO_CHECKSUM}" -eq 1 ]; then
        warn "--no-checksum: skipping BOTH SHA-256 and signature verification. This is dangerous."
        warn "Only continue if you obtained this build from a trusted, authenticated source."
        return 0
    fi

    # SHA256SUMS is mandatory: without it neither the archive nor the signature
    # can be validated, so a missing file is treated as an attack.
    local sums_url="${REPO_URL}/releases/download/v${VERSION}/SHA256SUMS"
    if ! curl -fsSL --max-time 30 "${sums_url}" -o "${TEMP_DIR}/SHA256SUMS" 2>/dev/null; then
        die "Could not fetch ${sums_url}. SHA256SUMS is mandatory for prebuilt installs. Refusing to install."
    fi
    if [ ! -s "${TEMP_DIR}/SHA256SUMS" ]; then
        die "Downloaded SHA256SUMS is empty. Refusing to install."
    fi

    # Signature verification runs regardless of which hashing tool exists, so a
    # missing sha256sum can never silently disable it.
    verify_signature "${TEMP_DIR}/SHA256SUMS"

    local expected actual
    if ! has_hash_tool; then
        die "No SHA-256 tool available (need sha256sum, shasum or openssl), so ${ASSET_FILE} cannot be verified. Install one, or re-run with --no-checksum (dangerous)."
    fi

    expected="$(awk -v name="${ASSET_FILE}" '
        { file = $2; sub(/^\*/, "", file); if (file == name) { print $1; exit } }
    ' "${TEMP_DIR}/SHA256SUMS")"
    if [ -z "${expected}" ]; then
        die "No checksum entry for ${ASSET_FILE} in SHA256SUMS. Refusing to install."
    fi

    actual="$(sha256_file "${TEMP_DIR}/${ASSET_FILE}")" \
        || die "Failed to compute SHA-256 for ${ASSET_FILE}. Refusing to install."
    if [ "${expected}" != "${actual}" ]; then
        die "Checksum mismatch for ${ASSET_FILE}. Refusing to install."
    fi
    ok "Checksum verified (${actual:0:12}...)."
}

extract_binary() {
    local archive="${TEMP_DIR}/${ASSET_FILE}"
    case "${ASSET_FILE}" in
        *.tar.gz) tar -xzf "${archive}" -C "${TEMP_DIR}" ;;
        *.zip)    unzip -q "${archive}" -d "${TEMP_DIR}" ;;
        *)        die "Unsupported archive format: ${ASSET_FILE}" ;;
    esac

    BINARY_SRC="${TEMP_DIR}/${PROJECT}"
    if [ ! -f "${BINARY_SRC}" ]; then
        die "Binary not found inside ${ASSET_FILE}. Expected: ${PROJECT}"
    fi
}

# ──────────────────────────────────────────────
# Install
# ──────────────────────────────────────────────

install_binary() {
    mkdir -p "${BIN_DIR}"
    # Stage next to the destination and rename: a failed copy must never
    # truncate an existing installation.
    local tmp_dest="${BIN_DIR}/.${PROJECT}.xfetch-tmp-$$"
    if command -v install >/dev/null 2>&1; then
        install -m 755 "${BINARY_SRC}" "${tmp_dest}" || { rm -f "${tmp_dest}"; die "Failed to copy the binary into ${BIN_DIR}."; }
    else
        cp "${BINARY_SRC}" "${tmp_dest}" || { rm -f "${tmp_dest}"; die "Failed to copy the binary into ${BIN_DIR}."; }
        chmod 755 "${tmp_dest}"
    fi
    mv -f "${tmp_dest}" "${BIN_DIR}/${PROJECT}"
    ok "Installed binary: ${BIN_DIR}/${PROJECT}"
}

install_config() {
    if [ "${FLAG_SKIP_CONFIG}" -eq 1 ]; then
        log "Skipping config generation (--skip-config)."
        return 0
    fi

    mkdir -p "${CONFIG_DIR}"

    if [ -f "${CONFIG_DIR}/config.jsonc" ]; then
        EXISTING_CONFIG=1
        warn "Config already exists at ${CONFIG_DIR}/config.jsonc — not overwriting."
    else
        if "${BIN_DIR}/${PROJECT}" --gen-config >/dev/null 2>&1; then
            ok "Generated config at ${CONFIG_DIR}/config.jsonc"
        else
            warn "Could not generate config; skipping."
        fi
    fi

    if [ "${OS_NAME}" = "macos" ]; then
        local mac_support="${HOME}/Library/Application Support/${PROJECT}"
        if [ ! -e "${mac_support}" ]; then
            ln -sf "${CONFIG_DIR}" "${mac_support}"
            ok "Created macOS config symlink: ${mac_support} -> ${CONFIG_DIR}"
        fi
    fi
}

# ──────────────────────────────────────────────
# PATH modification
# ──────────────────────────────────────────────

ensure_path_in_file() {
    local file="$1"
    local path_line="$2"
    local comment="$3"

    if [ ! -f "${file}" ]; then
        mkdir -p "$(dirname "${file}")"
        touch "${file}"
    fi

    if grep -qsF "# ${comment}" "${file}" 2>/dev/null; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && ok "PATH already configured in ${file}"
        return 0
    fi

    printf '\n# %s\n%s\n' "${comment}" "${path_line}" >> "${file}"
    ok "Added ${BIN_DIR} to PATH in ${file}"
}

modify_path() {
    local primary_rc
    primary_rc="$(detect_shell_rc)"

    local fish_rc="${HOME}/.config/fish/config.fish"
    local -a rc_list=()

    if [ "${OS_NAME}" = "macos" ]; then
        rc_list=("${HOME}/.bash_profile" "${HOME}/.zprofile" "${HOME}/.zshrc")
        [ -f "${HOME}/.bashrc" ] && rc_list+=("${HOME}/.bashrc")
    else
        rc_list=("${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile")
        [ -f "${HOME}/.bash_profile" ] && rc_list+=("${HOME}/.bash_profile")
    fi
    if [ -f "${fish_rc}" ] || [ "${primary_rc}" = "${fish_rc}" ]; then
        rc_list+=("${fish_rc}")
    fi

    local comment="${PROJECT} path"
    local path_line

    for rc in "${rc_list[@]}"; do
        if [ -f "${rc}" ] || [ "${rc}" = "${primary_rc}" ]; then
            if [ "${rc}" = "${fish_rc}" ]; then
                path_line="fish_add_path ${BIN_DIR}"
            else
                path_line="export PATH=\"${BIN_DIR}:\$PATH\""
            fi
            ensure_path_in_file "${rc}" "${path_line}" "${comment}"
        fi
    done

    if [ "${primary_rc}" = "${fish_rc}" ]; then
        ok "To add ${BIN_DIR} to your current session, run: fish_add_path ${BIN_DIR}"
    else
        ok "To add ${BIN_DIR} to your current session, run: source ${primary_rc}"
    fi
}

# ──────────────────────────────────────────────
# Verification / Summary
# ──────────────────────────────────────────────

verify_installation() {
    local binary="${BIN_DIR}/${PROJECT}"
    if [ ! -f "${binary}" ]; then
        error "Binary not found at ${binary}"
        return 1
    fi
    if [ ! -x "${binary}" ]; then
        error "Binary is not executable: ${binary}"
        return 1
    fi
    if "${binary}" --version >/dev/null 2>&1; then
        local version_output
        version_output="$("${binary}" --version 2>&1)"
        ok "Verified: ${version_output}"
    else
        warn "Binary installed at ${binary} but could not verify version."
    fi
    return 0
}

print_summary() {
    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} — Installation Complete (prebuilt)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  OS:              ${OS_NAME} (${ARCH_NAME})
  Version:         v${VERSION}
  Binary:          ${BIN_DIR}/${PROJECT}
  Config:          ${CONFIG_DIR}/

EOF

    if [ "${EXISTING_CONFIG}" -eq 1 ]; then
        cat <<EOF
    Existing config preserved at ${CONFIG_DIR}/config.jsonc
EOF
    fi

    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        local shell_rc
        shell_rc="$(detect_shell_rc)"
        if [ "${shell_rc}" = "${HOME}/.config/fish/config.fish" ]; then
            cat <<EOF
  PATH updated in:  ${shell_rc}
  Restart your terminal (or run 'fish_add_path ${BIN_DIR}') to use ${PROJECT}.
EOF
        else
            cat <<EOF
  PATH updated in:  ${shell_rc}
  Restart your terminal or run 'source ${shell_rc}' to use ${PROJECT}.
EOF
        fi
    else
        cat <<EOF
  PATH not modified. Add ${BIN_DIR} to your PATH manually:
    export PATH="${BIN_DIR}:\$PATH"
EOF
    fi

    cat <<EOF

  ${PROJECT} is ready! Run it:
    ${PROJECT}

  For configuration help:
    ${PROJECT} --help
    ${REPO_URL}

  To uninstall:
    rm -f "${BIN_DIR}/${PROJECT}"
    rm -rf "${CONFIG_DIR}"

  Uninstall script available at:
    ${REPO_RAW}/uninstall.sh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────

main() {
    parse_args "$@"

    if [ "${FLAG_NO_CHECKSUM}" -eq 1 ] && [ "${FLAG_REQUIRE_SIGNATURE}" -eq 1 ]; then
        die "--no-checksum and --require-signature are mutually exclusive."
    fi

    OS_NAME="$(detect_os)"
    ARCH_NAME="$(detect_arch)"
    resolve_target

    log "Installing ${PROJECT} (prebuilt) on ${OS_NAME} (${ARCH_NAME})"

    if [ "${EUID:-${UID}}" = "0" ] && [ "${FLAG_YES}" -eq 0 ]; then
        warn "Running as root is not recommended for local installs."
        if has_tty; then
            printf "[%s] Continue as root? [y/N]: " "${PROJECT}"
            read -r response
            case "${response}" in
                y|Y|yes) ;;
                *) die "Aborted. Run as a normal user, or use --yes to skip this check." ;;
            esac
        else
            die "Running as root without an interactive terminal. Re-run with --yes to skip this check."
        fi
    fi

    if ! command -v curl >/dev/null 2>&1; then
        die "curl is required. Install it and try again."
    fi
    if [ "${FLAG_NO_CHECKSUM}" -eq 0 ] && ! has_hash_tool; then
        warn "No SHA-256 tool found (sha256sum, shasum or openssl); install one."
        warn "The install will abort before verifying the archive unless --no-checksum is used (dangerous)."
    fi
    case "${OS_NAME}" in
        linux)   command -v tar >/dev/null 2>&1 || die "tar is required." ;;
        macos)   command -v tar >/dev/null 2>&1 || die "tar is required." ;;
    esac

    if ! mkdir -p "${BIN_DIR}" 2>/dev/null; then
        die "Cannot write to ${BIN_DIR}. Check HOME permissions."
    fi

    resolve_asset_url
    download_and_verify
    extract_binary
    install_binary
    install_config

    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        modify_path
    else
        log "Skipping PATH modification (--no-modify-path)."
    fi

    rm -rf "${TEMP_DIR}" 2>/dev/null || true
    TEMP_DIR=""

    verify_installation || die "Installation verification failed."
    print_summary
}

main "$@"
