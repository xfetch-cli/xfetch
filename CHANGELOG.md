# Changelog

## 2026-07-25 — v0.3.0

### Responsive Image & Text

-  Added `logo_width` / `logo_height` fields to Config for explicit image sizing in columns
-  Auto-responsive image width: when `logo_width` is unset, calculates from terminal size (28% width, clamped 15-40 cols)
-  Responsive text truncation: content lines exceeding available width (terminal - logo - gap) are truncated with `...`, preserving ANSI codes

### Extension API (separate from plugin system)

-  Created `api/crates/extension-api/` — standalone crate for extension protocol types (`xfetch-extension-api`)
-  Added `ConfigProviderRequest` / `ConfigProviderResponse` with its own `ExtensionKind` enum and `KIND_CONFIG_PROVIDER` constant
-  Added `ConfigProviderConfig` struct + `config_providers[]` field to `Config` — runs after theme merge, in declaration order
-  Created `xfetch/src/extensions/` module with `runner.rs` — invokes extension binaries via stdin/stdout JSON protocol, completely decoupled from `plugins/`
-  Created sample extension `layout-override` that rewrites `layout` / `modules` at load time (depends on `xfetch-extension-api`, not `xfetch-plugin-api`)

### Logo & Config Packs

-  Created `tests/` directory with 150 new configs (100 with logos, 50 without) at `~/.config/xfetch/tests/`
-  Replaced all 63 ASCII logos with high-quality art from asciiart.eu (artists: Joan G. Stark, Felix Lee, Shanaka Dias, Hayley Jane Wakenshaw, etc.)
-  Generated `test-commands.md` with 300 test commands for all configs
-  Created 12 city-themed color themes (x, madrid, lahabana, miami, paris, tokio, oslo, helsinki, berlin, london, praha, bogota)
-  Installed all 10 plugins (animate-logo, display-resolution, docker, github-stats, music-player, theme-detection, theme-manager, timezone, user-info, weather)

## 2026-07-24 — v0.2.0

### Theme System (Breaking Change)

-  Fixed merge order: theme now takes highest priority (defaults → config → theme)
-  deep_merge no longer overwrites non-empty strings with empty strings
-  Created berlin.jsonc as a proper monochromatic theme
-  Created THEMES.md with full theme system reference
-  Updated web docs (EN, ES, DE) with corrected merge order and migration guide

### Plugin Documentation

-  Created individual plugin reference pages under web/docs/{en,es,de}/plugins/
-  Streamlined xfetch/docs/PLUGINS.md to redirect to external plugin repository

### Roadmap

-  Closed items removed from scope (module scripting, conditional modules, theme variables)
-  Marked completed: linting (clippy), rustfmt, cross-platform tests, code coverage

### Version

-  Bumped to v0.2.0

## 2026-07-23

## Phase 0 · Foundation & Core

- Initialize Rust project with dependencies
- Cross-platform OS detection (Linux, Windows, macOS)
- System information gathering module
- Configuration system with JSONC support
- UI rendering engine with crossterm

## Phase 1 · System Information Modules

- OS name & architecture display
- Kernel version detection
- Hostname resolution
- Shell detection and display
- Terminal emulator detection
- CPU model & frequency information
- GPU detection (discrete & integrated)
- Memory and RAM usage display
- Disk usage statistics
- Battery status and percentage
- System uptime calculation
- Package count for multiple managers (pacman, dpkg, scoop)
- Desktop environment / window manager detection

## Phase 2 · Visual Customization & Layouts

- Custom ASCII art support from text files
- Image/SVG logo support via viuer
- ANSI color codes in ASCII logos
- Icon customization per module (Nerd Fonts)
- Color customization per module
- Default layout (side-by-side)
- Pac-Man layout with custom header/footer
- Side-block layout
- Tree layout for hierarchical display
- Section layout for grouped information
- Color palette display with style options

## Phase 3 · Documentation & Examples

- Installation guide
- Configuration guide
- Quick install script for Linux/macOS
- PowerShell install script for Windows
- 20+ example configurations
- Sample logos (text and SVG)
- Uninstallation scripts
- Layout documentation

## Phase 4 · Package Manager Expansion

- RPM package manager support (Fedora, RHEL)
- APK package manager support (Alpine)
- Nix package manager support
- Homebrew package manager support (macOS/Linux)
- Chocolatey package manager support (Windows)
- Multiple installed package manager detection
- Package count detection performance optimization

## Phase 5 · Network & Connectivity

- Local IP address detection
- Public IP address fetching (with privacy option)
- IPv6 support
- Network interface information display
- Option to disable IP fetching for privacy

## Phase 6 · Enhanced Modules

- Music player integration (MPD support)
- Spotify current track display
- Weather module with location API
- Timezone and world clock display
- User info and login status
- Display resolution and refresh rate
- Theme and color scheme detection

## Phase 7 · Additional Layouts

- Compact layout for minimal output
- Horizontal layout variant
- Bottom layout with logo below info
- Minimal layout (text-only)
- Layout preview documentation

## Phase 8 · Performance Optimization

- Parallelized slow hardware probes
- Module data caching
- GPU detection optimization for multi-GPU systems
- Lazy loading for optional modules
- Benchmarked and profiled performance
- Modularized file structure

## Phase 9 · CI/CD & Distribution

- GitHub Actions for automated builds
- Binary releases for Linux x86_64, macOS (Intel & ARM), and Windows
- AUR package for Arch Linux
- Homebrew tap for macOS
- Install scripts covering all platforms (Linux, macOS, Windows)
- Automated Rust installation in install.ps1 for Windows

## Phase 10 · Community & Ecosystem

- Themes repository and registry
- Theme download manager (plugin)
- Online theme preview tool
- Community theme contributions process
- Plugin system for custom modules
- Community issue templates for xfetch, plugins, configs, and api
- Contribution guidelines

## Phase 11 · Testing & Quality Assurance

- Unit tests for info module
- Unit tests for config module
- Integration tests for layouts
- 41 tests total, all passing

## Phase 12 · Advanced Features

> Out of respect for the privacy of our users: we have decided to eliminate even the possibility, we maintain this as a record of what should not be done under any circumstances in the future.

## Phase 13 · Documentation & Marketing

- Comprehensive user manual
- Project website with showcase
- Developer documentation
