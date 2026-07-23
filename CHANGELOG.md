# Changelog

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
