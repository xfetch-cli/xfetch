#!/usr/bin/env bash
# xfetch - cross-platform system information fetcher
# Installer script — supports remote (curl | bash) and local installation
# Usage: curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash
#        bash install.sh --local
#        bash install.sh --prefix /usr/local

set -euo pipefail
IFS=$'\n\t'

# ──────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────
REPO_URL="https://github.com/xfetch-cli/xfetch.git"
REPO_RAW="https://raw.githubusercontent.com/xfetch-cli/xfetch/main"

PROJECT="xfetch"
PROJECT_DESC="cross-platform system information fetcher"

# Default paths (may be overridden by flags)
PREFIX="${PREFIX:-${HOME}/.local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
CONFIG_DIR="${CONFIG_DIR:-${HOME}/.config/${PROJECT}}"
DATA_DIR="${DATA_DIR:-${PREFIX}/share/${PROJECT}}"

# Behavior flags
FLAG_LOCAL=0
FLAG_MODIFY_PATH=1
FLAG_YES=0
FLAG_VERBOSE=0
FLAG_SKIP_CONFIG=0
FLAG_SKIP_CARGO_INSTALL=0
FLAG_INSTALL_DEPS=0

# Runtime
TEMP_DIR=""
EXISTING_CONFIG=0
SHELL_RC_FILES=""

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
        FreeBSD) echo "freebsd" ;;
        OpenBSD) echo "openbsd" ;;
        NetBSD)  echo "netbsd" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)       echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        armv7l|armhf)  echo "armv7" ;;
        i686|i386)     echo "i686" ;;
        riscv64)       echo "riscv64" ;;
        *)             echo "unknown" ;;
    esac
}

detect_shell_rc() {
    # Detect the user's preferred shell config file
    local shell_name
    shell_name="$(basename "${SHELL:-${HOME}}" 2>/dev/null || echo "bash")"
    # Strip trailing version suffixes like zsh5 -> zsh
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
            if [ "$(detect_os)" = "macos" ]; then
                echo "${HOME}/.bash_profile"
            else
                echo "${HOME}/.bashrc"
            fi
            ;;
        fish) echo "${HOME}/.config/fish/config.fish" ;;
        *)
            # Fallback: check common rc files in order of preference
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
  curl -fsSL ${REPO_RAW}/install.sh | bash
  bash install.sh [options]

Options:
  -h, --help              Show this help message and exit
  -l, --local             Install from local source (skip git clone; run from repo root)
  -p, --prefix <dir>      Installation prefix (default: \${HOME}/.local)
  -b, --bin-dir <dir>     Binary install directory (default: \${PREFIX}/bin)
  -c, --config-dir <dir>  Config directory (default: \${HOME}/.config/${PROJECT})
  -n, --no-modify-path    Do not modify shell config files to add to PATH
  -y, --yes               Automatic yes to all prompts
  -s, --skip-config       Skip copying default config files
  -q, --quiet             Quiet mode (minimal output)
  -v, --verbose           Verbose output
  --no-cargo-install      Skip cargo install (assume binary already built)
  --install-deps          Install missing system dependencies automatically.
                          In interactive runs they are always offered; this flag
                          pre-authorizes it for non-interactive runs (CI, containers).
                          sudo is only used for this step, after confirmation.

Environment variables:
  PREFIX                  Same as --prefix
  BIN_DIR                 Same as --bin-dir
  CONFIG_DIR              Same as --config-dir
  DATA_DIR                Data files directory

Examples:
  # Quick install (remote, no sudo needed)
  curl -fsSL ${REPO_RAW}/install.sh | bash

  # Install missing system dependencies automatically (asks for sudo)
  curl -fsSL ${REPO_RAW}/install.sh | bash -s -- --install-deps

  # Non-interactive install (CI, remote without a TTY)
  curl -fsSL ${REPO_RAW}/install.sh | bash -s -- --yes

  # Local install from cloned repo
  bash install.sh --local

  # System-wide install
  bash install.sh --prefix /usr/local --yes

  # Install without PATH modification
  bash install.sh --no-modify-path

Report issues: ${REPO_URL}/issues
EOF
    exit 0
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help) usage ;;
            -l|--local) FLAG_LOCAL=1 ;;
            -n|--no-modify-path) FLAG_MODIFY_PATH=0 ;;
            -y|--yes) FLAG_YES=1 ;;
            -s|--skip-config) FLAG_SKIP_CONFIG=1 ;;
            -v|--verbose) FLAG_VERBOSE=1 ;;
            -q|--quiet) FLAG_VERBOSE=0 ;;
            --no-cargo-install) FLAG_SKIP_CARGO_INSTALL=1 ;;
            --install-deps) FLAG_INSTALL_DEPS=1 ;;
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
# Dependency checks
# ──────────────────────────────────────────────

detect_distro() {
    # Map /etc/os-release (with ID_LIKE fallbacks) to a package-manager family
    local id="" id_like=""
    if [ -r /etc/os-release ]; then
        id="$(sed -n 's/^ID=//p' /etc/os-release | tr -d '"' | head -n 1)"
        id_like="$(sed -n 's/^ID_LIKE=//p' /etc/os-release | tr -d '"' | head -n 1)"
    fi
    case "${id} ${id_like}" in
        *debian*|*ubuntu*)       echo "debian" ;;
        *arch*|*manjaro*|*endeavouros*) echo "arch" ;;
        *fedora*|*rhel*|*centos*|*rocky*|*alma*|*amzn*) echo "fedora" ;;
        *suse*|*opensuse*)       echo "suse" ;;
        *alpine*)                echo "alpine" ;;
        *void*)                  echo "void" ;;
        *gentoo*)                echo "gentoo" ;;
        *solus*)                 echo "solus" ;;
        *mageia*)                echo "mageia" ;;
        *slackware*)             echo "slackware" ;;
        *)                       echo "unknown" ;;
    esac
}

system_deps_cmd() {
    # Command that installs a C toolchain + git + curl on this distribution
    case "$(detect_distro)" in
        debian)   echo "apt-get update && apt-get install -y build-essential git curl" ;;
        arch)     echo "pacman -Sy --noconfirm base-devel git curl" ;;
        fedora)   echo "dnf install -y gcc gcc-c++ make git curl" ;;
        suse)     echo "zypper -n install -t pattern devel_basis && zypper -n install git curl" ;;
        alpine)   echo "apk add build-base git curl" ;;
        void)     echo "xbps-install -y base-devel git curl" ;;
        gentoo)   echo "emerge --ask=n --oneshot sys-devel/gcc sys-devel/binutils git curl" ;;
        solus)    echo "eopkg install -y -c system.devel git curl" ;;
        mageia)   echo "urpmi --auto git curl gcc gcc-c++ make" ;;
        slackware) echo "slapt-get -y -i git curl gcc binutils make" ;;
        *)        echo "" ;;
    esac
}

run_as_root() {
    # Run a command as root. Never prompts for a password without a TTY;
    # with a TTY, sudo reads the password from /dev/tty (works via curl | bash).
    if [ "${EUID:-${UID}}" = "0" ]; then
        "$@"
        return
    fi
    if ! command -v sudo >/dev/null 2>&1; then
        warn "sudo is not installed."
        return 1
    fi
    if ! sudo -n true 2>/dev/null && ! has_tty; then
        warn "sudo requires a password but there is no interactive terminal."
        return 1
    fi
    sudo "$@"
}

install_system_deps() {
    local missing=("$@")

    # macOS: CLT provides the C compiler; Homebrew provides git/curl
    if [ "$(detect_os)" = "macos" ]; then
        if ! command -v cc >/dev/null 2>&1 && ! xcode-select -p >/dev/null 2>&1; then
            error "Xcode Command Line Tools are missing."
            error "Run 'xcode-select --install' and accept the dialog, then re-run this installer."
            return 1
        fi
        if ! command -v brew >/dev/null 2>&1; then
            error "Homebrew is missing."
            error "Install it with: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            return 1
        fi
        local brew_pkgs=()
        command -v git >/dev/null 2>&1 || brew_pkgs+=(git)
        command -v curl >/dev/null 2>&1 || brew_pkgs+=(curl)
        if [ ${#brew_pkgs[@]} -gt 0 ]; then
            log "Installing via Homebrew: ${brew_pkgs[*]}"
            brew install "${brew_pkgs[@]}"
        fi
        ok "macOS dependencies ready."
        return 0
    fi

    local cmd
    cmd="$(system_deps_cmd)"
    if [ -z "${cmd}" ]; then
        error "Cannot auto-install dependencies on this distribution ($(detect_distro))."
        error "Install a C compiler, git and curl manually, then re-run."
        return 1
    fi

    if [ "${FLAG_YES}" -eq 0 ] && has_tty; then
        printf "[%s] Install missing dependencies with sudo? [y/N]\n[%s]   Command: sudo sh -c '%s'\n[%s] Proceed? [y/N]: " \
            "${PROJECT}" "${PROJECT}" "${cmd}" "${PROJECT}"
        read -r response
        case "${response}" in
            y|Y|yes) ;;
            *) die "Aborted. Run it manually: sudo sh -c '${cmd}'" ;;
        esac
    fi

    log "Installing system dependencies (sudo required)..."
    if ! run_as_root sh -c "${cmd}"; then
        error "Failed to install dependencies (sudo unavailable without a TTY?)."
        error "Run manually: sudo sh -c '${cmd}'"
        return 1
    fi
    ok "System dependencies installed."
}

check_deps() {
    local os
    os="$(detect_os)"

    if [ "${os}" = "windows" ]; then
        warn "Native Windows is supported via install.ps1 (PowerShell)."
        warn "Run: powershell -ExecutionPolicy Bypass -File install.ps1"
        die "Aborting: use install.ps1 on Windows."
    fi

    if [ "${FLAG_SKIP_CARGO_INSTALL}" -eq 0 ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            if [ "${FLAG_YES}" -eq 1 ] || ! has_tty; then
                warn "Rust (cargo) is not installed. Will attempt to install Rust via rustup..."
            else
                warn "Rust (cargo) is not installed."
                warn "The installer can install Rust via rustup for you."
                printf "[%s] Install Rust now? [Y/n]: " "${PROJECT}"
                read -r response
                case "${response}" in
                    n|N|no) die "Aborted. Please install Rust manually: https://rustup.rs/" ;;
                    *) ;;
                esac
            fi
            if ! command -v curl >/dev/null 2>&1; then
                die "curl is required to install Rust via rustup."
            fi
            install_rust
        fi
    fi

    # Collect missing system tools (git, curl, C compiler)
    local missing=()
    if [ "${FLAG_LOCAL}" -eq 0 ] && ! command -v git >/dev/null 2>&1; then
        missing+=("git")
    fi
    if ! command -v curl >/dev/null 2>&1; then
        missing+=("curl")
    fi
    if [ "${FLAG_SKIP_CARGO_INSTALL}" -eq 0 ] \
        && ! command -v cc >/dev/null 2>&1 \
        && ! command -v clang >/dev/null 2>&1; then
        missing+=("cc")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        warn "Missing system dependencies: ${missing[*]}"
        if [ "${FLAG_INSTALL_DEPS}" -eq 1 ] || has_tty; then
            # Interactive runs ask automatically; --install-deps pre-authorizes
            # non-interactive runs (CI, containers) to install them.
            if ! install_system_deps "${missing[@]}"; then
                die "Could not install system dependencies automatically."
            fi
        else
            if [ "${os}" = "macos" ]; then
                warn "macOS: install the Xcode Command Line Tools (xcode-select --install)."
            else
                local cmd
                cmd="$(system_deps_cmd)"
                if [ -n "${cmd}" ]; then
                    warn "Run: sudo sh -c '${cmd}'"
                else
                    warn "Install a C compiler, git and curl (the build-essential/base-devel equivalent of your distro)."
                fi
            fi
            warn "Or re-run with --install-deps (e.g. CI runs) to install them automatically."
            die "Missing required dependencies. Please install them and try again."
        fi
    fi

    # Check for the 'install' command (coreutils) on Linux
    if [ "${os}" != "macos" ]; then
        if ! command -v install >/dev/null 2>&1; then
            warn "coreutils 'install' command not found. Will use cp fallback."
        fi
    fi
}

preflight() {
    # HOME writability
    if ! mkdir -p "${BIN_DIR}" 2>/dev/null; then
        die "Cannot write to ${BIN_DIR}. Check HOME permissions."
    fi

    # Disk space (~2 GB for the Rust toolchain + build)
    local free_kb
    free_kb="$(df -k "${HOME}" 2>/dev/null | awk 'NR==2 {print $4}')"
    if [ -n "${free_kb}" ] && [ "${free_kb}" -lt 2097152 ] 2>/dev/null; then
        warn "Low disk space (${free_kb} KB free under ${HOME}). Building may fail."
    fi

    # Network reachability (warn only)
    if [ "${FLAG_LOCAL}" -eq 0 ]; then
        if ! curl -fsSL --max-time 8 -o /dev/null "https://github.com" 2>/dev/null; then
            warn "Cannot reach github.com — remote install may fail."
        fi
    fi
}

install_rust() {
    log "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y 2>/dev/null || die "Failed to install Rust."
    # Source the env so cargo is available immediately
    if [ -f "${HOME}/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "${HOME}/.cargo/env"
    fi
    ok "Rust installed successfully."
}

# ──────────────────────────────────────────────
# PATH modification
# ──────────────────────────────────────────────

ensure_path_in_file() {
    local file="$1"
    local path_line="$2"
    local comment="$3"

    if [ ! -f "${file}" ]; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && log "Creating ${file}..."
        mkdir -p "$(dirname "${file}")"
        touch "${file}"
    fi

    # Idempotency: bail out if our marker comment is already present
    if grep -qsF "# ${comment}" "${file}" 2>/dev/null; then
        [ "${FLAG_VERBOSE}" -eq 1 ] && ok "PATH already configured in ${file}"
        return 0
    fi

    printf '\n# %s\n%s\n' "${comment}" "${path_line}" >> "${file}"
    ok "Added ${BIN_DIR} to PATH in ${file}"
}

modify_path() {
    local os
    local primary_rc
    os="$(detect_os)"
    primary_rc="$(detect_shell_rc)"

    local fish_rc="${HOME}/.config/fish/config.fish"
    local -a rc_list=()

    # Determine which rc files to update
    if [ "${os}" = "macos" ]; then
        rc_list=("${HOME}/.bash_profile" "${HOME}/.zprofile" "${HOME}/.zshrc")
        # Also check for .bashrc on macOS (common with iTerm2)
        [ -f "${HOME}/.bashrc" ] && rc_list+=("${HOME}/.bashrc")
    else
        rc_list=("${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile")
        [ -f "${HOME}/.bash_profile" ] && rc_list+=("${HOME}/.bash_profile")
    fi
    # fish uses its own config file and PATH syntax
    if [ -f "${fish_rc}" ] || [ "${primary_rc}" = "${fish_rc}" ]; then
        rc_list+=("${fish_rc}")
    fi

    local comment="${PROJECT} path"
    local path_line

    for rc in "${rc_list[@]}"; do
        # Only modify files that exist (or the primary shell rc)
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
# Build and install
# ──────────────────────────────────────────────

build_project() {
    local src_dir="$1"

    log "Building ${PROJECT} (release mode)..."
    (cd "${src_dir}" && CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release --locked)
    ok "Build completed successfully."
}

install_binary() {
    local src_dir="$1"

    mkdir -p "${BIN_DIR}"

    local binary_src="${src_dir}/target/release/${PROJECT}"
    if [ ! -f "${binary_src}" ]; then
        # Try alternate location
        binary_src="${src_dir}/target/release/${PROJECT}.exe"
    fi
    if [ ! -f "${binary_src}" ]; then
        die "Binary not found after build. Expected: ${binary_src}"
    fi

    if command -v install >/dev/null 2>&1; then
        install -m 755 "${binary_src}" "${BIN_DIR}/${PROJECT}"
    else
        cp "${binary_src}" "${BIN_DIR}/${PROJECT}"
        chmod 755 "${BIN_DIR}/${PROJECT}"
    fi

    ok "Installed binary: ${BIN_DIR}/${PROJECT}"
}

install_config() {
    local src_dir="$1"

    if [ "${FLAG_SKIP_CONFIG}" -eq 1 ]; then
        log "Skipping config installation (--skip-config)."
        return 0
    fi

    mkdir -p "${CONFIG_DIR}"

    # Check if config already exists
    if [ -f "${CONFIG_DIR}/config.jsonc" ]; then
        EXISTING_CONFIG=1
        warn "Config already exists at ${CONFIG_DIR}/config.jsonc — not overwriting."
    else
        # Generate the first config with the freshly installed binary
        # (adds the distro ASCII logo when online; falls back offline).
        if command -v "${PROJECT}" >/dev/null 2>&1; then
            if "${PROJECT}" --gen-config >/dev/null 2>&1; then
                ok "Generated config at ${CONFIG_DIR}/config.jsonc"
            else
                warn "Could not generate config; skipping."
            fi
        else
            warn "Binary not found on PATH; skipping config generation."
        fi
    fi

    # macOS: create symlink for Library/Application Support
    local os
    os="$(detect_os)"
    if [ "${os}" = "macos" ]; then
        local mac_support="${HOME}/Library/Application Support/${PROJECT}"
        if [ ! -e "${mac_support}" ]; then
            ln -sf "${CONFIG_DIR}" "${mac_support}"
            ok "Created macOS config symlink: ${mac_support} -> ${CONFIG_DIR}"
        fi
    fi
}

# ──────────────────────────────────────────────
# Verification
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

    # Try running the binary
    if "${binary}" --version >/dev/null 2>&1; then
        local version_output
        version_output="$("${binary}" --version 2>&1)"
        ok "Verified: ${version_output}"
    else
        warn "Binary installed at ${binary} but could not verify version."
        warn "It may still work; run '${binary}' directly to test."
    fi

    return 0
}

print_summary() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
${PROJECT} — Installation Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  OS:              ${os} (${arch})
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

    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    log "Installing ${PROJECT} on ${os} (${arch})"

    # Check we're not running as root unnecessarily (for local installs)
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

    check_deps

    preflight

    # Determine source directory
    local src_dir=""
    if [ "${FLAG_LOCAL}" -eq 1 ]; then
        src_dir="$(pwd)"
        if [ ! -f "${src_dir}/Cargo.toml" ]; then
            die "No Cargo.toml found in ${src_dir}. Run --local from the project root."
        fi
        log "Using local source: ${src_dir}"
    else
        TEMP_DIR="$(mktemp -d)"
        src_dir="${TEMP_DIR}/${PROJECT}"

        log "Cloning repository from ${REPO_URL}..."
        git clone --depth 1 "${REPO_URL}" "${src_dir}"
        ok "Repository cloned."
    fi

    # Build
    if [ "${FLAG_SKIP_CARGO_INSTALL}" -eq 1 ]; then
        log "Skipping cargo build (--no-cargo-install). Checking for pre-built binary..."
        if [ ! -f "${src_dir}/target/release/${PROJECT}" ] \
            && [ ! -f "${src_dir}/target/release/${PROJECT}.exe" ]; then
            die "No pre-built binary found. Remove --no-cargo-install to build from source."
        fi
        ok "Using pre-built binary."
    else
        build_project "${src_dir}"
    fi

    # Install binary
    install_binary "${src_dir}"

    # Install config
    install_config "${src_dir}"

    # Modify PATH
    if [ "${FLAG_MODIFY_PATH}" -eq 1 ]; then
        modify_path
    else
        log "Skipping PATH modification (--no-modify-path)."
    fi

    # Clean up temp dir (if we cloned)
    if [ -n "${TEMP_DIR}" ] && [ "${FLAG_LOCAL}" -eq 0 ]; then
        rm -rf "${TEMP_DIR}" 2>/dev/null || true
        TEMP_DIR=""
    fi

    # Verify
    verify_installation || die "Installation verification failed."

    # Summary
    print_summary
}

main "$@"
