<h1>Installation Guide</h1>

<p>
  This guide covers the complete installation process for <strong>xfetch</strong> on Linux, macOS, and Windows.
</p>

<hr>

<h2>Prerequisites</h2>

<p>
  The installer follows a <strong>no-sudo-by-default</strong> design: everything is installed in
  your user space (<code>~/.local/bin</code>, <code>~/.config/xfetch</code>), and <code>sudo</code>
  is only ever used to install missing system dependencies, always after asking for your
  confirmation first. Nothing else requires privileges.
</p>

<ul>
  <li><strong>bash</strong> — present on virtually every Linux/macOS system (required to run the script itself)</li>
  <li><strong>curl</strong> — used by the remote one-liner and by rustup</li>
  <li><strong>git</strong> — for cloning the repository (not needed for the remote one-liner)</li>
  <li><strong>Rust/Cargo</strong> — the build toolchain. If not installed, the installer can set it up via <a href="https://rustup.rs/">rustup</a>, fully in user space (no sudo)</li>
  <li><strong>C compiler</strong> — a C toolchain is required to link Rust binaries: <code>build-essential</code> (Debian/Ubuntu), <code>base-devel</code> (Arch), <code>devel_basis</code> pattern (openSUSE), Xcode Command Line Tools (macOS), etc.</li>
</ul>

<p>
  If any of these are missing, the installer asks you in the terminal and installs them for you
  automatically (using your distro's package manager, with sudo only for that step). In
  non-interactive environments (CI, containers) it never uses sudo without authorization: either
  pass <code>--install-deps</code> or run the printed command yourself.
</p>

<hr>

<h2>Quick Install (Recommended)</h2>

<p>
  The fastest way to install xfetch. <strong>No sudo is required</strong> — if your system already
  has a C toolchain, this installs xfetch entirely in your user space. If something is missing
  (Rust, C compiler, git, curl), the installer asks and handles it automatically.
</p>

<h3>Linux / macOS</h3>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash</code></pre>

<p>
  Prefer no toolchain? The prebuilt installer downloads the verified binary from
  the latest release (no Rust, no compilation):
</p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install-prebuilt.sh | bash</code></pre>

<p>
  If <code>cargo</code> is not installed, the script will offer to install Rust via rustup automatically.
</p>

<p>
  If system dependencies are missing, it will ask for your confirmation and then prompt for your
  sudo password to install them (only that step uses sudo):
</p>

<p>
  In non-interactive environments (CI, containers), pass <code>--install-deps</code> to pre-authorize
  the dependency installation:
</p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash -s -- --install-deps</code></pre>

<h3>WSL</h3>

<p>
  WSL is treated as a normal Linux system by the installer — no special steps are needed.
  If your WSL distribution runs as root by default, pass <code>--yes</code> to skip the
  root confirmation prompt.
</p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash -s -- --yes</code></pre>

<h3>Windows (PowerShell)</h3>

<p>
  Prebuilt (no Rust needed):
</p>

<pre><code class="language-powershell">irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install-prebuilt.ps1 | iex</code></pre>

<p>
  From source (installs Rust via rustup when missing, then builds):
</p>

<pre><code class="language-powershell">irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.ps1 | iex</code></pre>

<h3>What the Script Does</h3>

<ol>
  <li>Preflight checks: writable home, disk space, network reachability</li>
  <li>Checks for Rust (offers to install it if missing, via rustup, no sudo)</li>
  <li>Clones the repository</li>
  <li>Builds the binary with <code>cargo build --release --locked</code></li>
  <li>Installs it to <code>~/.local/bin/</code></li>
  <li>Sets up default config files in <code>~/.config/xfetch/</code></li>
  <li>Adds <code>~/.local/bin</code> to your PATH (via <code>~/.bashrc</code>, <code>~/.zshrc</code>, <code>~/.zprofile</code>, or <code>config.fish</code>)</li>
</ol>

<h3>Install Script Options</h3>

<p>
  The install script supports several flags for customization:
</p>

<pre><code class="language-bash"># Install missing system dependencies automatically (prompts for sudo)
bash &lt;(curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh) --install-deps

# Install to a custom prefix
bash &lt;(curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh) --prefix /usr/local

# Skip PATH modification
bash &lt;(curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh) --no-modify-path

# Install from a local clone of the repository
git clone https://github.com/xfetch-cli/xfetch.git
cd xfetch
bash install.sh --local

# Non-interactive install (auto-yes to prompts)
bash install.sh --local --yes</code></pre>

<p>
  For all available flags:
</p>

<pre><code class="language-bash">bash &lt;(curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh) --help</code></pre>

<hr>

<h2>Local Install</h2>

<p>
  If you have already cloned the repository, run the installer directly from the project root:
</p>

<pre><code class="language-bash">cd xfetch
bash install.sh --local</code></pre>

<p>
  This skips the git clone step and builds from your local copy.
</p>

<hr>

<h2>Build from Source (Manual)</h2>

<p>
  For full control over the build:
</p>

<pre><code class="language-bash"># Clone
git clone https://github.com/xfetch-cli/xfetch.git
cd xfetch

# Build release binary
cargo build --release

# The binary is at: target/release/xfetch
# Install it manually:
cp target/release/xfetch ~/.local/bin/

# Generate a starter config (adds the distro ASCII logo when online)
xfetch --gen-config</code></pre>

<p>
  Additional presets are available in
  <a href="https://github.com/xfetch-cli/configs">xfetch-cli/configs</a>.
</p>

<hr>

<h2>Install via Cargo</h2>

<p>
  Install the latest release straight from <a href="https://crates.io/crates/xfetch-cli">crates.io</a>
  (the installed command is <code>xfetch</code>):
</p>

<pre><code class="language-bash">cargo install xfetch-cli</code></pre>

<p>
  This installs to <code>~/.cargo/bin/</code> (ensure it is in your PATH).
</p>

<p>
  Alternatively, install from a local clone of the repository:
</p>

<pre><code class="language-bash">cargo install --path .</code></pre>

<hr>

<h2>macOS (Homebrew)</h2>

<p>
  The easiest way to install xfetch on macOS is via
  <a href="https://brew.sh/">Homebrew</a>. No Rust toolchain or compilation is
  needed — a prebuilt binary is downloaded and verified against its SHA256
  checksum. It works on both Apple Silicon (arm64) and Intel (x86_64) Macs.
</p>

<pre><code class="language-bash">brew tap xfetch-cli/tap
brew install xfetch</code></pre>

<p>
  To update to a newer release:
</p>

<pre><code class="language-bash">brew upgrade xfetch</code></pre>

<p>
  To uninstall:
</p>

<pre><code class="language-bash">brew uninstall xfetch</code></pre>

<p>
  <strong>Note:</strong> because Homebrew installs the binary directly (without
  macOS quarantine), no code signing or Gatekeeper bypass is required.
</p>

<hr>

<h2>Arch Linux (AUR)</h2>

<p>
  xfetch is packaged in the <a href="https://aur.archlinux.org/packages?O=0&amp;K=xfetch">Arch User Repository</a>
  as <code>xfetch-core-bin</code> (precompiled binary) and <code>xfetch-git</code>
  (builds from source). With an AUR helper such as <code>yay</code>:
</p>

<pre><code class="language-bash">yay -S xfetch-core-bin   # precompiled binary
yay -S xfetch-git        # build from source</code></pre>

<p>
  Without an AUR helper, clone the package repository and build it manually:
</p>

<pre><code class="language-bash">git clone https://aur.archlinux.org/xfetch-core-bin.git
cd xfetch-core-bin
makepkg -si</code></pre>

<p>
  Installs system-wide to <code>/usr/bin/xfetch</code>.
</p>

<p>
  To uninstall the package:
</p>

<pre><code class="language-bash">sudo pacman -R xfetch-core-bin</code></pre>

<p>
  Packages are maintained in the AUR, not in the xfetch repository, so update
  them with <code>pacman -Syu</code> (or your AUR helper); <code>xfetch update</code>
  never replaces a package-manager install.
</p>

<hr>

<h2>Verifying Installation</h2>

<p>
  After installing, verify xfetch works:
</p>

<pre><code class="language-bash">xfetch --version</code></pre>

<p>
  You should see version output. Then run it to test the display:
</p>

<pre><code class="language-bash">xfetch</code></pre>

<h3>Troubleshooting &quot;command not found&quot;</h3>

<p>
  If you get a &quot;command not found&quot; error:
</p>

<ul>
  <li><strong>Restart your terminal</strong>, or</li>
  <li>Run <code>source ~/.bashrc</code> (or <code>source ~/.zshrc</code>), or</li>
  <li>Manually add <code>~/.local/bin</code> to your PATH:</li>
</ul>

<pre><code class="language-bash">export PATH="$HOME/.local/bin:$PATH"</code></pre>

<hr>

<h2>Updating</h2>

<p>
  xfetch can check for and install new releases itself:
</p>

<pre><code class="language-bash">xfetch update --check   # only report; exits 1 when a newer release exists
xfetch update           # install the newest release
xfetch update --yes     # skip the confirmation prompt</code></pre>

<p>
  The command detects how the binary was installed before touching anything:
</p>

<ul>
  <li>
    <strong>Prebuilt installs</strong> (<code>install-prebuilt.sh</code>,
    typically <code>~/.local/bin</code>): the matching release asset is
    downloaded, verified against the published <code>SHA256SUMS</code>,
    extracted and moved over the current executable with an atomic rename. A
    single <code>xfetch.bak</code> keeps the previous binary and is
    overwritten on the next update.
  </li>
  <li>
    <strong>cargo installs</strong> (<code>~/.cargo/bin</code>): updated with
    <code>cargo install xfetch-cli --force --locked</code>.
  </li>
  <li>
    <strong>Package-manager installs and local builds</strong>: never replaced;
    the command prints the right command instead.
  </li>
</ul>

<p>
  <code>xfetch update --prebuilt --bin-dir &lt;dir&gt;</code> forces the
  in-place update of <code>&lt;dir&gt;/xfetch</code> (Unix only). On Windows
  the prebuilt path is not available yet; use
  <code>cargo install xfetch-cli --force</code> or your installer of choice.
</p>

<p>
  <code>GH_TOKEN</code> or <code>GITHUB_TOKEN</code> raise the GitHub API rate
  limit, and <code>XFETCH_UPDATE_API</code> overrides the release endpoint
  (useful for mirrors and tests).
</p>

<h2>Uninstallation</h2>

<p>
  See the <a href="UNINSTALLATION.md">Uninstallation Guide</a> for detailed instructions.
</p>

<p><strong>Quick uninstall:</strong></p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash</code></pre>

<p><strong>Uninstall including config files and PATH entries:</strong></p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash -s -- --purge</code></pre>

<p><strong>Manual removal:</strong></p>

<pre><code class="language-bash">rm -f ~/.local/bin/xfetch
rm -rf ~/.config/xfetch</code></pre>

<hr>

<h2>Next Steps</h2>

<ul>
  <li><a href="CONFIGURATION.md">Configuration Guide</a> — customize modules, logos, colors, and layouts</li>
  <li><a href="GEN_CONFIG.md">Config Generation Guide</a> — generate a starter config with the distro logo and your layout</li>
  <li><a href="LAYOUTS.md">Layouts Guide</a> — explore built-in display layouts</li>
  <li><a href="PLUGINS.md">Plugins Guide</a> — extend xfetch with external plugins</li>
</ul>
