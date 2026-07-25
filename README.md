<h1 align="center">
<img src="https://raw.githubusercontent.com/xfetch-cli/assets/main/logo/banner/xfetch.svg" width="100%" alt="XFetch banner" /></h1>

<div align="center">

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/xfetch-cli/xfetch/blob/main/LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20windows-lightgrey?style=flat-square)](https://github.com/xfetch-cli/xfetch/blob/main/docs/INSTALLATION.md)
[![Build](https://img.shields.io/github/actions/workflow/status/xfetch-cli/xfetch/rust-tests.yml?style=flat-square&logo=github&label=build)](https://github.com/xfetch-cli/xfetch/actions/workflows/rust-tests.yml)

<p>A cross-platform system information fetching tool written in Rust.</p>

<a href="https://imgur.com/8qcIITv">
<img src="https://i.imgur.com/fRD6bgX.gif" width="900" alt="Demo" >
</a>

</div>

<!--Menu-->
<div align="left">
  <h2>Menu</h2>
  <ul>
    <li><a href="#previews">Previews <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/device-camera.svg"/></a></li>
    <li><a href="#quick-install">Quick Install </a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/pass-filled.svg"/></li>
    <li><a href="#features">Features </a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/grabber.svg" /></li>
    <li><a href="#quick-install">Installation </a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/github-action.svg" /></li>
    <li><a href="#configuration">Configuration </a> <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/gear.svg" /></li>
    <li><a href="#usage">Usage </a> <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/flame.svg"/></li>
    <li><a href="#related-documents">Related Documents </a>
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/diff-multiple.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/repo-forked.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/github-alt.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/key.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/git-merge.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/symbol-keyword.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/symbol-key.svg" />
    <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/file-submodule.svg" />
    </li>
    <li><a href="#related-repos">Related Repos </a> <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/symbol-key.svg"/></li>
    <li><a href="#about-the-developer">About X </a> <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/regex.svg"/></li>
  </ul>
</div>


<!-- previews-->
</div>

<h2  id="previews" align="center"> Previews</h2>

<p align="center">
  <a href="./assets/previews/preview0.jpg">
    <img src="https://raw.githubusercontent.com/xscriptor/xassets/main/xrepos/apps/xfetch/preview03.jpg" alt="Main preview" width="850"/>
  </a>
</p>

<details>
  <summary>More previews</summary>

  <table align="center">
    <tr>
      <td align="center">
        <img src="https://raw.githubusercontent.com/xscriptor/xassets/main/xrepos/apps/xfetch/preview02.png" alt="Preview 2" width="490"/>
      </td>
      <td align="center">
        <img src="https://raw.githubusercontent.com/xscriptor/xassets/main/xrepos/apps/xfetch/preview01.jpg" alt="Preview 3" width="490"/>
      </td>
    </tr>
  </table>
</details>


<h2 id="quick-install" align="center"> Quick Install</h2>

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.ps1 | iex
```

> For detailed installation steps — prerequisites, manual builds, package managers, and uninstallation — see the [Installation Guide](docs/INSTALLATION.md).

<h2 id="features" align="center"> Features</h2>

- **Cross-platform**: Works on Linux, Windows, and macOS.
- **Customizable**: Configure modules via `config.jsonc`.
- **Fast**: Written in Rust for performance.

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
xfetch
```

<h2 id="related-documents" align="center">Related Documents</h2>

<div align="left">
  <ul>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/INSTALLATION.md">Installation</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/CONFIGURATION.md">Config</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/LAYOUTS.md">Layouts</a></li>
    <li><a href="https://github.com/xfetch-cli/xfetch/blob/main/docs/PLUGINS.md">Plugins</a></li>
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
  <li><a href="https://github.com/xfetch-cli/api">XFetch API</a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/git-fetch.svg"/></li>
  <li><a href="https://github.com/xfetch-cli/configs">XFetch Configs </a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/symbol-key.svg"/></li>
  <li><a href="https://github.com/xfetch-cli/plugins">XFetch Plugins </a><img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/extensions.svg"/></li>
</ul>


<div id="about-the-developer" align="center">
<h2>X</h2>

<a href="https://dev.xscriptor.com">
  <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/verified-filled.svg" width="24" alt="X Web" />
</a>
 & 
<a href="https://github.com/xscriptor">
  <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/github.svg" width="24" alt="X Github Profile" />
</a>
 & 
<a href="https://www.xscriptor.com">
  <img src="https://xscriptor.github.io/icons/icons/code/product-design/xsvg/quotes.svg" width="24" alt="Xscriptor web" />
</a>

</div>
