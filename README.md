<h1 align="center">
<img src="https://raw.githubusercontent.com/xfetch-cli/assets/main/logo/banner/xfetch.svg" width="100%" alt="XFetch banner" /></h1>

<div align="center">

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/xfetch-cli/xfetch/blob/main/LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20windows-lightgrey?style=flat-square)](https://github.com/xfetch-cli/xfetch/blob/main/docs/INSTALLATION.md)
[![Build](https://img.shields.io/github/actions/workflow/status/xfetch-cli/xfetch/rust-tests.yml?style=flat-square&logo=github&label=build)](https://github.com/xfetch-cli/xfetch/actions/workflows/rust-tests.yml)

<p>A cross-platform system information fetching tool written in Rust.</p>

<a href="https://xfetch-cli.github.io/web/previews">
<img src="https://xfetch-cli.github.io/web/previews/xfetch-demo.gif" width="900" alt="Demo" >
</a>

</div>

<!--Menu-->
<div align="left">
  <h2>Menu</h2>
  <ul>
    <li><a href="#previews">Previews </a></li>
    <li><a href="#quick-install">Quick Install </a></li>
    <li><a href="#features">Features </a></li>
    <li><a href="#quick-install">Installation </a></li>
    <li><a href="#configuration">Configuration </a> </li>
    <li><a href="#usage">Usage </a> </li>
    <li><a href="#related-documents">Related Documents </a>
    </li>
    <li><a href="#related-repos">Related Repos </a> </li>
    <li><a href="#about-the-developer">About X </a> </li>
  </ul>
</div>


<!-- previews-->
</div>

<h2  id="previews" align="center"> Previews</h2>

<p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-2.webp" alt="Preview Linux" width="850"/>
  </p>

<details>
  <summary>More... </summary>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-1.gif" alt="Preview Linux" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-3.webp" alt="Preview Linux" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-4.webp" alt="Preview Linux" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-5.webp" alt="Preview Linux" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-linux-6.webp" alt="Preview Linux" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-windows-1.webp" alt="Preview Windows" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-windows-2.webp" alt="Preview Windows" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/preview-windows-3.webp" alt="Preview Windows" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/macos-preview-1.gif" alt="Preview macOS" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/macos-preview-2.webp" alt="Preview macOS" width="850"/>
  </p>

  <p align="center">
    <img src="https://xfetch-cli.github.io/web/previews/macos-preview-3.webp" alt="Preview macOS" width="850"/>
  </p>
</details>


<h2 id="quick-install" align="center"> Quick Install</h2>

**Linux / macOS** (no sudo required):
```bash
curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash
```

The installer asks automatically for anything missing (Rust, C compiler, git, curl) — sudo is
only used to install those dependencies, after your confirmation. For non-interactive runs (CI):
```bash
curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash -s -- --install-deps
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.ps1 | iex
```

**Cargo (crates.io):**
```bash
cargo install xfetch-cli
```

**macOS (Homebrew):**
```bash
brew tap xfetch-cli/tap && brew install xfetch
```

**Arch Linux (AUR, via yay):**
```bash
yay -S xfetch-core-bin   # precompiled binary
yay -S xfetch-git        # build from source
```

> For detailed installation steps — prerequisites, manual builds, package managers, and uninstallation — see the [Installation Guide](docs/INSTALLATION.md).

<h2 id="features" align="center"> Features</h2>

- **Cross-platform**: Works on Linux, Windows, and macOS.
- **Customizable**: Configure modules, layouts, icons and colors via `config.jsonc`.
- **Fast**: Written in Rust for performance.
- **Animated logos**: Animate the ASCII logo with plugins (e.g. `animate-logo`).
- **Effects**: Installable intro animations that reveal the info with style (e.g. a "decrypt" effect) — opt-in via `"effects"` in the config.
- **Daemon mode**: Pin an animated fetch at the top of the terminal and keep using the shell below.
- **Live stats daemon**: Turn the pinned fetch into a live panel — re-probes cpu/memory/battery/... every few seconds (`daemon_live`), with hot reload of the config on the fly (`daemon_live_reload`).
- **Themes, plugins & extensions**: Switch visual themes, extend info with plugins, and transform the config with extensions.

<h2 id="configuration" align="center"> Configuration </h2>

xfetch looks for a configuration file at:

- **Linux**: `~/.config/xfetch/config.jsonc`
- **Windows**: `%APPDATA%\xfetch\config.jsonc`
- **macOS**: `~/Library/Application Support/xfetch/config.jsonc`

Additional curated presets and example configs are maintained in
[`xfetch-cli/configs`](https://github.com/xfetch-cli/configs).

### Example Config (`config.jsonc`)

```jsonc
// Configuration for xfetch
{
  // Path to custom ASCII art file (optional)
  "ascii": null, 
  // Modules to display
  "modules": [
    "os",
    "kernel",
    "wm",
    "packages",
    "shell",
    "cpu",
    "gpu",
    "memory",
    "disk",
    "battery",
    "uptime",
    "terminal"
  ],
  // Enable colors
  "show_colors": true
}
```

<h2 id="usage" align="center"> Usage</h2>

Simply run `xfetch` in your terminal.

```bash
xfetch                          # render the fetch
xfetch --daemon                 # pin an animated fetch at the top (daemon mode)
xfetch --daemon-stop            # stop the daemon
xfetch --no-daemon-live         # disable the live stats daemon (config: "daemon_live": true)
xfetch --daemon-live-stop       # stop the live stats daemon
xfetch --daemon-live-reload     # hot reload the live stats daemon ("daemon_live_reload": true)
xfetch --config path/to/config.jsonc
xfetch --gen-config             # generate a starter config
xfetch --clean-cache            # clear the module cache
xfetch plugin install <name>    # install a plugin
xfetch extension install <name> # install an extension
xfetch theme list               # list themes
xfetch effects install <name>   # install an intro effect (e.g. decrypt)
```

> Full documentation: see the docs below.

<h2 id="related-documents" align="center">Related Documents</h2>

<div align="left">
  <ul>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/INSTALLATION.md">Installation</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/CONFIGURATION.md">Config</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/submodules_configuration.md">Submodule Config (labels &amp; formats)</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/DAEMON.md">Daemon Mode</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/LAYOUTS.md">Layouts</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/PLUGINS.md">Plugins</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/EFFECTS.md">Effects</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/EXTENSIONS.md">Extensions</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/UNINSTALLATION.md">Uninstall</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/ROADMAP.md">Roadmap</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/LICENSE">License</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/CONTRIBUTING.md">Contributing</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/SECURITY.md">Security</a></li>
  </ul>
</div>

<p align="center"><em>Contribute to the project, report issues, or connect with the developer using the links around</em></p>

<h2 align="center" id="related-repos">Related Repos</h2>
<ul>
  <li><a href="https://github.com/xfetch-cli/api">XFetch API</a></li>
  <li><a href="https://github.com/xfetch-cli/configs">XFetch Configs </a></li>
  <li><a href="https://github.com/xfetch-cli/plugins">XFetch Plugins </a></li>
</ul>


<div id="about-the-developer" align="center">
<h2>X</h2>

<a href="https://xscriptor.io">Dev</a>
 & 
<a href="https://github.com/xscriptor">Git</a>
 & 
<a href="https://www.xscriptor.com">X</a>

</div>
