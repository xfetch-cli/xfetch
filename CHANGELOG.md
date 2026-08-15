# Changelog

## 2026-08-15 — v0.4.0

### XDG_CONFIG_HOME Support (macOS fix)

- Added `config_dir()` helper in `src/config.rs`: prefers an absolute `XDG_CONFIG_HOME`, falling back to `dirs::config_dir()` — on macOS with `XDG_CONFIG_HOME` set, configs are now read directly from `~/.config/xfetch/` instead of `~/Library/Application Support/xfetch/`.
- Applied the helper to `default_config_path()`, `default_themes_dir()`, `resolve_theme_path()`, plugin lookup (`default_plugin_dir`), extension lookup (`default_extension_dir`, `find_extension_binary`), and daemon state files (`daemon.pid`, `daemon.rows`).
- Added `config_search_dirs()`: looks up the XDG dir first, then falls back to the platform default dir. `find_plugin_binary()`, `find_extension_binary()`, and `resolve_theme_path()` search both, so existing installs in the legacy dir keep working without reinstalling.
- Fixed daemon silent exit on macOS: with `XDG_CONFIG_HOME` set, the daemon previously could not find plugins installed in the legacy dir, so `prepare_frames()` returned `None` and `--daemon` exited with code 0 without forking. The legacy-dir fallback restores plugin discovery.
- Behavior on Linux and Windows is unchanged: without `XDG_CONFIG_HOME`, `config_search_dirs()` resolves to a single directory (the same one `dirs::config_dir()` returns).

### Section-Box Layout (`"layout": "section-box"`)

- New layout that renders a bordered box per module group: `╭─ Title ─╮` header, `│` rows, `╰───╯` footer, with the title embedded in the top border.
- Each box measures its own content width (ANSI-stripped) so borders align per group; groups are separated by a blank line.
- Nested groups render as boxes inside boxes (recursive `render_group_box`); top-level modules outside groups render as plain lines.
- Added `render_section_box()`, `render_group_box()`, `render_section_row()` in `src/ui/renders.rs` and registered the layout in `src/ui/layout.rs` — existing layouts are untouched.
- Added unit test `test_render_section_box_groups` covering border presence and alignment.

### Custom-X Layout (`"layout": "custom-x"`)

- New fully customizable layout: every border line is a literal template the user writes in the `custom_x` config object — top, bottom, left, right, group titles, internal dividers, and extra header/footer lines.
- Templates support two placeholders: `{fill}` repeats the `fill` character until the line reaches the box width (templates without `{fill}` are extended with `fill`), and `{title}` is replaced with the current group title.
- `divider_between` controls internal separators (`"groups"`, `"modules"`, or `"none"`); `padding` controls the space between borders and content; group title and divider templates are independent.
- `module_top` / `module_bottom` wrap every module row in its own box (rendered around each line), so a single outer frame can group everything while each module stays individually enclosed.
- Fixed row alignment: content rows now pad inside the right border, so `left`/`right` borders land exactly on the box edge.
- New `custom_x.width` option: `"auto"` (default, content-sized), `"full"` (stretches the box to the end of the terminal line, accounting for the logo column), or a fixed number of columns.
- New `custom_x.full_margin` option (default 2): how many cells `"full"` keeps free at the right edge of the terminal, avoiding the wrap column on narrow terminals; the small-terminal content fallback from `print_output` is now also applied when computing the stretched width, keeping both paths consistent.
- Added `src/ui/custom_x.rs` (new module with its own types and renderer, 7 unit tests) and a new `custom_x` field on `Config` (additive, defaults to `None`); registered the layout in `src/ui/layout.rs` — no existing layout or config behavior changed.

## 2026-08-13 — v0.4.0

### Daemon Mode (`--daemon`)

- **Daemon mode** (`--daemon` / `"daemon": true`): forks to the background, writes its PID to `~/.config/xfetch/daemon.pid` and exits immediately — the shell prompt returns instantly while the animation loops pinned at the top. Stop it with `xfetch --daemon-stop`.
- Pins the logo in a fixed scroll region (`SetScrollRegion`/`ResetScrollRegion`) and draws each row with absolute `MoveTo(0,row)` + cursor save/restore, so command output scrolls below the pinned block.
- Each frame is emitted as a single atomic `write_all` and the user's cursor is restored after every frame — typed input is never yanked away.
- The scroll region is re-asserted every frame (the shell may reset it) and `SIGWINCH` resizes are detected, recomputing geometry and redrawing.
- `stop_daemon()` validates the PID via `/proc/<pid>/comm` before signaling (avoids killing a recycled PID); the daemon also writes `daemon.rows` (pinned height) next to `daemon.pid`.
- The child uses `setsid()`, installs SIGINT/SIGTERM/SIGHUP handlers and exits cleanly (cursor shown, scroll region reset, PID file removed); the module is gated with `#[cfg(unix)]`.
- Added `src/ui/daemon.rs` and `src/ui/frames.rs` (shared, unit-tested frame loading); refactored `src/ui/print.rs` to share `compute_frame_geometry()`/`render_frame()` between the one-shot animation and the daemon; added `SetScrollRegion`/`ResetScrollRegion` ANSI commands and a direct `libc` dependency.

## 2026-07-25 — v0.3.0 (evening)

### Image Rendering Overlap Fix (Kitty)

- Diagnosed root cause: viuer renders kitty images with `z=0` (on top of text) using inaccurate cell ratio (1:2 vs 1:2.2+), and the cursor was mismanaged because kitty protocol doesn't advance the cursor after image placement
- Replaced `MoveUp`/`MoveToColumn` pattern with `SavePosition`/`RestorePosition` in `get_logo_data()` — cursor always returns to the correct starting row regardless of protocol used
- Added `logo_gap: Option<u32>` config field — configurable gap between image and text (default 12 cols, user configs set to 3)
- Added `logo_kitty: Option<bool>` config field — toggles native kitty protocol (`true`, default) vs half-block rendering (`false`) for kitty terminals
- Fixed `print_stacked_output()` to handle image-only logos (was ignoring `image_printed=true` with empty `ascii_lines`, causing text to overlap image in vertical layouts)
- Improved auto-logo-width: `0.28 * term_width` (was `0.18`), clamped `[12, 42]` (was `[12, 30]`)
- Removed absolute `MoveTo(0, bottom)` cursor positioning at end of print functions that assumed output started at row 0
- Gap calculation in `print_output()` and `print_animated_output()` now uses `config.logo_gap`
- Added overlap detection fallback: if text column leaves < 10 cols for content, reduces gap to minimum usable
- Updated all 100 image-based config-roulette configs with `logo_gap: 3` and `logo_kitty: true`
- Added `image` crate dependency (already transitive via viuer, now direct for potential future use)
- Explored and rejected custom kitty renderer with `z=-1` due to kitty cell reservation behavior

## 2026-07-25 — v0.3.0 (morning)

### Responsive Image & Text

-  Added `logo_width` / `logo_height` fields to Config for explicit image sizing in columns
-  Auto-responsive image width: when `logo_width` is unset, calculates from terminal size (28% width, clamped 15-40 cols)
-  Responsive text truncation: content lines exceeding available width (terminal - logo - gap) are truncated with `...`, preserving ANSI codes

### Extension API (separate from plugin system)

-  Created `api/crates/extension-api/` — standalone crate for extension protocol types (`xfetch-extension-api`)
-  Added `ConfigProviderRequest` / `ConfigProviderResponse` with its own `ExtensionKind` enum and `KIND_CONFIG_PROVIDER` constant
-  Added `ConfigProviderConfig` struct + `config_providers[]` field to `Config` — runs after theme merge, in declaration order
-  Created `xfetch/src/extensions/` module with `runner.rs` — invokes extension binaries via stdin/stdout JSON protocol, completely decoupled from `plugins/`
-  Extension binaries searched in `~/.config/xfetch/extensions/` (fallback to `plugins/`), prefixed `xfetch-extension-*`
-  Renamed `ConfigProviderConfig.plugin` → `ConfigProviderConfig.extension` to match the concept
-  Switched `xfetch-extension-api` dependency from local path to git remote (`github.com/xfetch-cli/api`)
-  Added `xfetch extension install/list/remove` commands (mirrors plugin commands, installs to `~/.config/xfetch/extensions/`)

### Extensions Built

-  Created sample extension `layout-override` that rewrites `layout` / `modules` at load time
-  Created `config-roulette` extension — reads a JSON list of config file paths, picks one (random or daily strategy), loads the full config file, and returns it; supports 300+ routes from the test suite
-  Created `extensions/` repo skeleton with workspace, .gitignore, and README matching plugins repo style
-  Each extension includes a detailed README with usage, args table, and protocol reference

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
