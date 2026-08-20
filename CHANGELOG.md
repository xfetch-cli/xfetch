# Changelog

## 2026-08-20 — v0.7.0

### Configurable Labels and Value Formats

- New `labels` config map renames the key shown per module (`"cpu": "procesador"`), in **every layout** (classic and variants, compact, minimal, section, section-box, tree, custom-x). An empty string hides the key — icon-only row. Colors keep using the raw module key, so renaming never breaks the color.
- New `formats` config map replaces a module's value with a template: `{field}` placeholders are substituted per module. Every module exposes `{value}` and `{key}`; structured modules add more fields:
  - CPU: `{brand}`, `{model}`, `{cores}`, `{freq}`
  - GPU: `{name}`, `{vendor}`, `{model}`, `{vram}`
  - memory/swap: `{used}`, `{total}`, `{percent}`; disk adds `{fs}`
  - os: `{distro}`, `{version}`, `{arch}`, `{wsl}`
  - packages: one field per manager (`{pacman}`, `{aur}`, ...), plus `{count}`, `{manager}`, `{managers}`
  - battery: `{percent}`, `{state}`; uptime: `{days}`, `{hours}`, `{mins}`; datetime: `{date}`, `{time}`
  - Unknown fields render empty; `{{` and `}}` escape literal braces.
- No hardcoded output: the default template is `{value}` for every module, so existing configs and output are byte-for-byte unchanged. Formatting happens once in `prepare_render_tree` (`info/format.rs`), so every layout and the daemons (animated-logo and live) see the same values.
- GPU field extraction is per platform, where the probe output lives: Linux parses the `lspci` bracket description, Windows the `Name`/CIM value, macOS the `system_profiler` chipset model (`platform/<os>/gpu.rs`); the shared rules (vendor detection, VRAM, model cleaning) live in `platform/shared/gpu.rs` with tests.
- Documented in `docs/submodules_configuration.md` (linked from CONFIGURATION.md and the README).
- 141 tests, clippy clean.

## 2026-08-19 — v0.6.0

### Themes

- `theme set` no longer rewrites the whole config: it edits only the `theme` key, preserving comments, formatting and the rest of the file (single quotes and bare keys supported).
- Theme format simplified: themes declare only what they change. Icons were removed from theme files and from `theme export` — they are a per-user font choice, filled from the defaults. New `logo_color` field colors the ASCII logo; `colors` already covers any module key, including `plugin:<name>` entries.
- New `logo_colors` field: per-row logo coloring (array of colors, cycled by row) — works for static logos and animation frames.
- 86 tests, clippy clean.

### Windows: Package Counter and Probes

- `winget` now counts only packages installed via winget (`--source winget`); before it counted every registered app (ARP/MSIX included). Chocolatey was removed from the core probes (it comes back as a plugin later).
- Shell detection walks the parent process chain instead of trusting `PSModulePath`, so `cmd.exe` is no longer reported as PowerShell; `local_ip` prefers the physical adapter (vEthernet/WSL/Hyper-V skipped); Windows version → logo mapping uses build numbers (`10.0.17763` is no longer matched as Windows 7).
- GPU/battery/datetime probes hardened (`-NoProfile -NonInteractive`, UTF-8 output, exit-status checks); `expand_path("~")` no longer panics; the localized winget "no packages" message is no longer counted; `--daemon` prints a warning on Windows; zero-capacity disks are skipped.

### Plugin and Extension Timeouts

- New top-level `subprocess.rs` (shared by probes, plugins and extensions): bounded pipe drains — grandchildren holding the pipe (winget COM server, etc.) can no longer hang xfetch — plus stdin support for the plugin protocol and process-tree kill (Windows via `taskkill` in `platform/windows/process.rs`).
- Optional per-plugin `timeout_secs` in the config (`info_plugins`, `config_providers`, `logo_animation`): the core kills the process when exceeded. Opt-in — behavior is unchanged when unset. Plugins/extensions also declare their own budget via the new `with_timeout` API helper (see xfetch-cli/api).
- All Windows-specific logic is consolidated under `src/info/platform/windows/` (`battery`, `datetime`, `gpu`, `network`, `packages`, `process`, `shell`, `software`, `version`); `info/software.rs` and `info/system.rs` dispatch to it.
- 79 tests, clippy clean.

### macOS and Linux: Per-Platform Modularization

- **macOS** now mirrors the Windows layout: `platform/macos/version.rs` (version → logo mapping, e.g. macOS 15 → sequoia), `platform/macos/software.rs` (`get_shell_info`/`get_desktop_info`, Aqua no longer inline in `info/software.rs`), `platform/macos/network.rs` (`local_ip` prefers the physical adapter — utun/tap/bridge/AWDL/Tailscale skipped).
- **Linux** gets `platform/linux/os_release.rs` (`/etc/os-release` parsing moved out of `logos.rs`) and `platform/linux/network.rs` (`local_ip` prefers the physical adapter — docker0/veth/br-/virbr/tun/tap/Tailscale skipped).
- **Arch package split**: `pacman` counts only official packages (`pacman -Qn`) and a new `aur` entry counts AUR/manual installs (`pacman -Qm`). Running `yay -Qq`/`paru -Qq` alongside pacman double- or triple-counted the same set (AUR packages are installed via pacman); helpers are no longer probed. The pacman database fast-path was dropped since it holds official + AUR together. Gentoo `portage` count is now displayed (it was computed but never surfaced). `PackageCheck` became a struct with a distinct `label`, Linux-only. No changes to other distros.
- `logos.rs` has no `#[cfg(target_os)]` anymore: `detect_os_ids` and `logo_category` moved to `platform/mod.rs`; the Windows logo mapping still lives in `platform/windows/version.rs` (unchanged).
- `get_local_ip_info` is dispatched through `platform::get_local_ip_info` like the other probes; `info/system.rs` no longer carries per-OS `#[cfg]` blocks. Windows behavior is untouched.
- 94 tests, clippy clean (Linux CI unchanged).

### Live Stats Daemon (`daemon_live`)

- New live stats daemon, sibling of the animated-logo daemon (`ui/daemon.rs` is untouched): pins the fetch at the top of the terminal and re-probes a lightweight module subset every `daemon_live_refresh` seconds, re-rendering with fresh values. Activation is config-only (`"daemon_live": true`); `--no-daemon-live` disables it from the terminal and `--daemon-live-stop` stops it (own pid file `daemon_live.pid`).
- Per-OS refresh policy lives in the new `platform/<os>/live.rs`: Linux refreshes 7 modules every 2 s, macOS every 3 s, Windows every 5 s (battery excluded by default — it spawns `wmic`/PowerShell). `daemon_live_modules` overrides the set.
- When `logo_animation` is configured the logo keeps animating while the content refreshes live; otherwise it is static. The engine lives in `ui/live.rs` and reuses the existing `print.rs` builders and probes — no existing rendering/probe code changed.
- **Hot reload**: `daemon_live_reload` (or `--daemon-live-reload`) watches the config file and the active theme (mtime) and re-applies modules, colors, layout, logo and refresh cadence without restarting; config providers (extensions) re-run on each reload. Disabled by default.
- 103 tests, clippy clean.

### Effects (Intro Animations)

- New **effects** category: installable intro animations over the info lines. The core renders the content, sends it to an effect binary (`xfetch-effect-<name>`, protocol `xfetch-effect-api`), and plays the returned frames before settling on the final content. Opt-in via `"effects"` in the config — a missing binary or bad response falls back to the plain fetch (no behavior change).
- New `xfetch-effect-api` crate in `xfetch-cli/api` (`EffectRequest`/`EffectResponse`, validation, IO/timeout helpers) + a reference `decrypt` effect in `crates/effect-api/examples/decrypt-effect.rs`. The `xfetch-cli/effects` repo will host effect implementations.
- Core wiring: `src/effects/` (binary resolution + runner), `"effects"` config block, `print_effect_output` (plays effect frames over the content, then settles), and `ui/logo.rs::build_logo_frames` (shared logo-frame builder, also used by the live daemon). Nothing existing was removed.
- 106 tests, clippy clean.

### Effects Repository

- New `xfetch-cli/effects` repository: workspace mirroring `plugins` (one crate per effect, binary `xfetch-effect-<name>`). The `decrypt` effect lives there (`effects/decrypt`) as the reference implementation of the protocol.
- New CLI subcommand `xfetch effects install|list|remove` (mirrors `plugin`): `install` builds from a local path or clones the effects repo (default `github.com/xfetch-cli/effects`, override with `--repo`/`XFETCH_EFFECT_REPO`) and copies the binary to `~/.config/xfetch/effects/`.
- Docs: `docs/EFFECTS.md` (install, config, protocol, writing effects).

### Effects: Chaining + Glitch + Shared Lib

- `"effects"` now accepts a **single effect or a list** — effects play in sequence (glitch → decrypt → ...). The core runs each effect on the rendered lines and plays the frames one after another before settling (untagged deserialization keeps existing single-object configs working).
- New `glitch` effect in `xfetch-cli/effects` (`effects/glitch`): stuttery scrambled flicker with deterministic "corruption bursts", settling on the real text.
- New shared crate `effects/effects-lib` (`xfetch-effects-lib`) in the effects repo: ANSI-safe tokenizer + reveal helpers, used by `decrypt` and `glitch` (no effect reimplements the tokenizer).
- 106 tests core + effects tests, clippy clean.

## 2026-08-18 — v0.5.0

### Per-Platform Package Counters

- **Windows**: `winget` support added (`winget list --include-unknown --disable-interactivity --accept-source-agreements`, 20 s timeout for slow first runs); `choco list --local-only` no longer miscounts — its "X packages installed." summary line is skipped; `scoop list` counts rows instead of subtracting a fixed header offset.
- **macOS**: `brew list --formula` output is parsed defensively (empty lines, `==> ...` notices and any whitespace noise are ignored), fixing counts that broke on Homebrew output variations.
- **Linux**: `yay` and `paru` (Arch AUR helpers) added to the probes, alongside `pacman`.
- Separation is kept per OS folder (`platform/{linux,macos,windows}/packages.rs`), each deciding commands, args and timeouts; the pure output parsers moved to `shared/packages.rs` (`count_scoop_output`, `count_choco_output`, `count_winget_output`, `count_brew_output`) so their tests run on any platform's local CI — +4 tests, 82 total.
- Cleaned up cfg gating so the cross-target checks (`cargo check --target` for Windows and macOS) are warning-free: Linux-only machinery (`PackageCheck`, `run_package_checks`, `run_package_check_with_timeout`) is `cfg(target_os = "linux")`.

## 2026-08-18 — v0.5.0

### No More `configs/` Folder

- The `configs/` directory was removed from the repo: the `--gen-config` template is now embedded in the binary (`GEN_CONFIG_TEMPLATE` const), and the installers (`install.sh` / `install.ps1`) generate the first config by running `xfetch --gen-config` instead of copying a file.
- `--gen-config` now ships the **`section`** layout by default (the same default the installers used to copy: grouped Hardware/Software/Session modules); `--layout pacman` and any other layout remain available via the flag. Offline installs fall back to the template without logo, and existing configs are never overwritten.
- `docs/INSTALLATION.md` manual setup example updated to `xfetch --gen-config`.

### Layout Flag in `--gen-config`

- New `--layout <name>` flag: generates the config with a different layout, e.g. `xfetch --gen-config --layout pacman` or `--layout tree`. Accepts any known layout (`default`, `side-block`, `tree`, `section`, `section-box`, `custom-x`, `compact`, `minimal`, `pacman`, `box`, `line`, `dots`, `bottom_line`, `horizontal`, `bottom`); the template ships as `section` (see the `configs/` removal entry above), so that remains the default.
- Unknown layout name: warns and keeps `pacman`. Composable with `--logo` (`--gen-config --layout tree --logo arch`).
- `LAYOUT_NAMES`/`is_known_layout()` exported from `ui/layout.rs` as the single source of truth for valid names.
- 78 tests, fmt and clippy `-D warnings` clean.

### Version Bump

- Crate version bumped from 0.4.0 to **0.5.0**: all 2026-08-18 entries below are part of this release (performance rounds, WSL presentation, package managers, parallel plugins, distro logos and the `--logo` flag).

### Logo Override in `--gen-config`

- New `--logo <id>` flag: embeds a specific logo in the generated config, overriding the detected OS/distro — e.g. `xfetch --gen-config --logo arch` on Ubuntu, or `--logo windows-11` / `--logo macos-ventura` for other OSes. Resolved on its own against the catalog (ids and aliases, case-insensitive).
- Unknown `--logo` id: warns and falls back to the generic logo of the current category, saved as `default.txt`; total failure (no network, catalog error) warns and writes the template unchanged. Automatic detection keeps its silent fallback.
- 78 tests, fmt and clippy `-D warnings` clean.

## 2026-08-18 — v0.5.0

### Distro Logos in `--gen-config`

- `--gen-config` now fetches the ASCII logo of the detected OS/distro from the new **xfetch-cli/logos** catalog (`raw.githubusercontent.com/xfetch-cli/logos/main`): index → resolution → raw art file, via `curl` with a 10 s timeout, validated (64 KB cap, no NUL bytes, sane line width).
- Detection: Linux reads `ID` + `ID_LIKE` from `/etc/os-release` (resolution: exact ID → each ID_LIKE token in order → category default, all case-insensitive against ids and aliases); macOS and Windows resolve their base id plus a version-specific logo when `os_version()` maps (`macos-ventura`, `windows-11`, ...).
- The fetched art is persisted to `<config_dir>/xfetch/logos/<distro-id>.txt` and the generated config gets `"ascii": "<path>"` (the existing config field for ASCII art files). `XFETCH_LOGOS_URL` overrides the base URL (testing forks/mirrors).
- **Fallback**: any failure (no network, catalog error, invalid art, missing file) writes the template unchanged — the previous behavior, including no runtime/daemon changes. Verified end-to-end: Ubuntu art renders in the pacman layout; unreachable URL falls back with no `ascii` field.
- Tests: +6 (os-release parsing, entry resolution by id/alias/ID_LIKE/case, art validation) — 78 total, all passing; fmt and clippy `-D warnings` clean.

## 2026-08-18 — v0.5.0

### Smarter sysinfo Initialization (Probe Slimming)

- `System::new_all() + refresh_all()` replaced with `System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()).with_memory(MemoryRefreshKind::everything()))`: the old call also walked every process — which xfetch never reads — measured ~40× slower on WSL (43 ms vs 1 ms). `os`, `kernel`, `hostname` and `uptime` are sysinfo statics and no longer require a `System` instance at all; only `cpu`, `memory` and `swap` create one.
- The three sysinfo containers (`System`, `Disks`, `Networks`) are now initialized concurrently in a `thread::scope` instead of serially.
- Benchmark label updated to reflect the parallel section contents (`Parallel (probes)`).
- Measured on WSL: cold full fetch 0.054 s → **0.008 s** total (originally 8.6 s). Values unchanged: `716 (dpkg)`, CPU/memory/swap/disk, `2026-08-18 10:18:27`, `Ubuntu 24.04 x86_64 (WSL)`.
- 72 tests, fmt and clippy `-D warnings` clean.

## 2026-08-18 — v0.5.0

### WSL Presentation, More Package Managers, Parallel Plugins

- **WSL-aware OS display** — new `src/info/platform/wsl/` folder (compiled on Linux, dispatched at runtime since WSL *is* Linux). `is_wsl()` reads `/proc/version` (kernel announces "microsoft", works for every distro inside WSL: Ubuntu, Debian, Arch, openSUSE, Alpine WSL, ...); `decorate_os()` applies the configured style. New config key `os_wsl_style`: `off` ("Ubuntu 24.04 x86_64"), `minimal` (default: "... (WSL)"), `full` ("... (WSL 2, WSLg)" — WSL version from the kernel string, WSLg from `/mnt/wslg` or `WAYLAND_DISPLAY`). Unknown config keys are ignored, so existing configs are unaffected.
- **Void and Gentoo package support** — new db-reads: Void (`/var/db/xbps/` per-package dirs, dot-prefixed partial installs excluded) and Gentoo (`/var/db/pkg/<category>/<package>/`, two-level count, matches `qlist -I`). Void also gets an `xbps-query -l` command fallback. Coverage now spans Debian/Ubuntu, Arch, Alpine, Fedora/RHEL/openSUSE (rpm command, binary db), Flatpak, Snap, Nix, Void and Gentoo.
- **Plugins run in parallel — API untouched** — `load_plugin_info()` spawns one thread per plugin (each plugin is an independent subprocess, so no shared state) and the whole load moved into the main parallel section of `Info::new`. Verified: two 1 s plugins complete in ~1.2 s instead of ~2.1 s serial. The `xfetch_plugin_api` JSON protocol is unchanged; no existing plugin needs any modification.
- Tests: +4 WSL (`is_wsl` consistency, `decorate_os` off/no-op, style fallback), 72 total, all passing; fmt and clippy `-D warnings` clean.

## 2026-08-18 — v0.5.0

### Performance Round 2: Database Reads, PATH Pre-check, Full Parallelism

- **Package counts from distro databases instead of subprocesses** (`linux/packages.rs`): `dpkg` (`/var/lib/dpkg/status` `Package:` lines), `pacman` (`/var/lib/pacman/local/` dirs), `apk` (`/var/lib/apk/db/installed` `P:` lines) and `flatpak` (system `/var/lib/flatpak/app/` + user `~/.local/share/flatpak/app/` dirs) are read directly — world-readable files that mirror exactly what `dpkg --get-selections`, `pacman -Qq`, `apk info` and `flatpak list --app` report, in microseconds instead of a spawn. The commands remain as automatic fallback when a database is unreadable or missing; `rpm`, `snap` and `nix-env` stay command-based (binary databases / profile symlinks). Breakdown order and format are unchanged.
- **PATH pre-check before spawning any probe** (`shared/commands.rs`): `run_cmd_with_timeout()` now verifies the binary is reachable through PATH before spawning, skipping the execvp search. WSL Windows mounts (`/mnt/c`, `/mnt/d`, ...) are excluded from the search: stat()ing those 9p/drvfs mounts makes a failed spawn cost hundreds of milliseconds. Semantics match execvp for all remaining PATH entries, so no probe that would succeed is ever skipped; applies to every command probe (GPU, battery, datetime, packages) on Unix.
- **`battery` and `datetime` moved into the parallel section** (`info/mod.rs`): previously fetched serially after the GPU/packages/IP block, each with its own 10 s timeout; now they join the parallel scope (a real win on macOS `pmset`/`system_profiler` and Windows `wmic`/`powershell`, which are slow).
- **Public IP hosts queried in parallel** (`system.rs`): `ifconfig.me`, `api.ipify.org` and `icanhazip.com` used to run sequentially (up to 9 s offline at 3 s per host); now all three fire at once and the first success wins.
- Measured on WSL (snapd off): cold full fetch 0.26 s → **0.047 s**; cold `packages` module 0.23 s → **0.002 s** (originally 8.7 s). Output is identical: `716 (dpkg)`, `1 hour, 44 mins`, `2026-08-18 09:47:28`, etc.
- Tests: +5 (`is_windows_mount`, `binary_reachable`, `count_lines_with_prefix`, `count_dirs`, `db_counts_preserve_check_order`) — 68 total, all passing; fmt and clippy `-D warnings` clean.

## 2026-08-18 — v0.5.0

### Package Counter Speedup

- Fixed `run_package_checks()` running every package-manager probe in series: the per-check `handle.join()` inside `filter_map` waited for each command before spawning the next, so cold package counts took the sum of all probes instead of the slowest one. All probes are now spawned first and joined after, cutting cold time on multi-manager systems dramatically.
- Removed the duplicated package scan: `get_packages_info()` (which re-ran the whole breakdown internally) plus a second `get_packages_breakdown()` call in `src/info/mod.rs` meant every cold fetch ran all package checks twice. The breakdown now runs once and the display string is derived from it via `packages_info_from_breakdown()`. The redundant `get_packages_info()` was deleted.
- Snap socket pre-check: on systems where `snap` is installed but the snapd daemon is not running (e.g. WSL), `snap list` blocks forever on the snapd socket and costs the full 3 s timeout per scan. `get_packages_breakdown()` on Linux now skips the `snap` probe entirely when neither `/run/snapd.socket` nor `/run/snapd-snap.socket` exists — an instant, subprocess-free check; the count would be zero anyway. Snap still counts where snapd actually runs.
- Windows `run_package_checks_with_adjustment()` got the same spawn-all-then-join fix.
- Measured on a WSL system with snapd off: cold `packages` module 8.7 s → 0.23 s (~38× faster); the earlier 3 s `snap` timeout is gone and the parallel section no longer runs probes twice.
- No config, layout, theme, or daemon behavior was changed. Tests updated: `test_get_packages_not_empty` builds the string from the breakdown, new `test_snapd_socket_precheck_skips_snap` guards the socket probe — 63 tests, all passing.

## 2026-08-17 — v0.4.0

### External Command Timeouts (hang fix)

- Fixed xfetch hanging before rendering on systems where `snap` is installed but the snapd daemon is not running (e.g., WSL): `snap list` blocks forever on the snapd socket, and `count_packages_linux()` waited on that thread inside `thread::scope`, so the whole fetch never printed.
- Added `run_cmd_with_timeout()` in `src/info/platform/shared/commands.rs`: runs a command with piped stdout/stderr and a deadline, kills the process when it expires and returns `None` instead of blocking forever; stdin is closed (`Stdio::null`) so commands can no longer wait on terminal input.
- Timeouts are per command, carried as a `(&str, &[&str], Duration)` tuple (`PackageCheck`), so each platform tunes its own commands independently: `snap` 3 s (the pathological case), package managers (`pacman`, `dpkg`, `rpm`, `flatpak`, `apk`, `nix-env`, `brew`, `scoop`, `choco`) 10 s, hardware probes (`lspci`, `pmset`, `wmic`, `powershell`) 10 s, `system_profiler` 30 s (macOS, notoriously slow), `date` 10 s.
- Windows: the `wmic` → `powershell` GPU/battery fallback now also triggers when `wmic` times out (previously only on spawn errors).
- Added cross-platform test `test_cmd_timeout_kills_hanging_command` (`sleep` on Unix, `timeout` on Windows) proving a hanging command is killed within the deadline — 62 tests total, all passing.
- No config, layout, theme, plugin, extension, or daemon behavior was changed.

### Platform Separation (`src/info/platform/`)

- New per-OS folder structure: `src/info/platform/{linux,macos,windows}/` plus `shared/` for platform-agnostic machinery — the first step toward per-platform code that is only compiled on its own OS.
- Contract: every OS folder exposes the same functions (`get_gpu_info()`, `get_battery_info()`, `get_datetime_info()`, `get_packages_breakdown()`), and `platform/mod.rs` re-exports the active one via `#[cfg(target_os)]`; the rest of the code calls `info::*` and never sees the OS.
- Moved GPU detection (lspci / system_profiler / wmic+powershell), battery (sysfs / pmset / wmic), datetime (date / powershell) and the per-OS package-manager tables into their platform folders.
- `shared/` keeps only the mechanism: `commands.rs` (command runner with timeout) and `packages.rs` (probe runner, `PackageCheck` type, per-command timeouts, `get_packages_info`).
- `hardware.rs` now holds only cross-platform sysinfo probes (cpu, memory, swap, disk); `software.rs` only env-var detection (shell, terminal, desktop, user); `system.rs` only sysinfo/network probes (os, kernel, hostname, uptime, IPs).
- The unused `Components` battery probe was dropped (the battery value no longer depends on it).
- Tests moved with the code: per-OS detector tests (`test_linux/macos/windows_detectors_safe`), per-OS datetime tests, package-machinery tests in `shared/packages.rs`, and command-runner tests in `shared/commands.rs`.

### Local CI

- Added `scripts/ci.sh` (Linux/macOS) and `scripts/ci.ps1` (Windows): run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` locally.
- GitHub Actions workflows (`rust-tests.yml`, `roadmap-sync.yml`) now trigger only manually (`workflow_dispatch`) — no runs on push.
- Fixed pre-existing rustfmt drift (`cargo fmt --all`) and clippy warnings in `ui/logo.rs`, `ui/nodes.rs`, `ui/renders.rs`, `ui/daemon.rs` so the local CI passes cleanly.

## 2026-08-15 — v0.4.0

### XDG_CONFIG_HOME Support (macOS fix)

- Added `config_dir()` helper in `src/config.rs`: prefers an absolute `XDG_CONFIG_HOME`, falling back to `dirs::config_dir()` — on macOS with `XDG_CONFIG_HOME` set, configs are now read directly from `~/.config/xfetch/` instead of `~/Library/Application Support/xfetch/`.
- Applied the helper to `default_config_path()`, `default_themes_dir()`, `resolve_theme_path()`, plugin lookup (`default_plugin_dir`), extension lookup (`default_extension_dir`, `find_extension_binary`), and daemon state files (`daemon.pid`, `daemon.rows`).
- Added `config_search_dirs()`: looks up the XDG dir first, then falls back to the platform default dir. `find_plugin_binary()`, `find_extension_binary()`, and `resolve_theme_path()` search both, so existing installs in the legacy dir keep working without reinstalling.
- Fixed daemon silent exit on macOS: with `XDG_CONFIG_HOME` set, the daemon previously could not find plugins installed in the legacy dir, so `prepare_frames()` returned `None` and `--daemon` exited with code 0 without forking. The legacy-dir fallback restores plugin discovery.
- Behavior on Linux and Windows is unchanged: without `XDG_CONFIG_HOME`, `config_search_dirs()` resolves to a single directory (the same one `dirs::config_dir()` returns).

### Section-Box Layout (`"layout": "section-box"`)

- New layout that renders a bordered box per module group: `╭─ Title ─╮` header, `│` rows, `╰───╯` footer, with the title embedded in the top border.

### Keys and Logo Options

- New `show_keys` config option (default `false`): renders `key: value` in the icon-style layouts (classic, section, compact, custom-x, box variants) instead of only the icon — opt-in, no visual change for existing configs.
- New `key_width` config option: pads the key to a fixed number of columns so values align vertically (applies wherever keys are shown, including `section` and `minimal`).
- New logo options: `logo_color` (ANSI color name applied to the ASCII logo), `logo_padding` (leading spaces before the logo), and `logo_type` (`"auto"` by extension, `"ascii"` forces text rendering, `"image"` forces image rendering).
- `logo_color` now supports names (`"Cyan"`), 256-color indexes (`"196"`), and hex RGB (`"#FF0000"`) via the new `color_sgr()` helper, and it also applies to animated logos (plugin frames) through the new `logo::apply_logo_style()` — previously animation frames bypassed the color.
- Added `color_code_from_name()` helper in `src/ui/renders.rs` and unit tests for keys and logo options (61 tests total).
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
