<h1>Installation Guide</h1>

<p>
  This guide covers the complete installation process for <strong>xfetch</strong> on Linux, macOS, and Windows.
</p>

<hr>

<h2>Prerequisites</h2>

<ul>
  <li><strong>git</strong> — for cloning the repository (not needed for the remote one-liner install)</li>
  <li><strong>Rust/Cargo</strong> — the build toolchain. If not installed, the installer can set it up via <a href="https://rustup.rs/">rustup</a></li>
  <li><strong>macOS</strong>: Xcode Command Line Tools (<code>xcode-select --install</code>)</li>
  <li><strong>Linux</strong>: <code>build-essential</code>, <code>pkg-config</code>, <code>libssl-dev</code> (Debian/Ubuntu) or <code>base-devel</code> (Arch)</li>
</ul>

<hr>

<h2>Quick Install (Recommended)</h2>

<p>
  The fastest way to install xfetch.
</p>

<h3>Linux / macOS</h3>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.sh | bash</code></pre>

<p>
  If <code>cargo</code> is not installed, the script will offer to install Rust via rustup automatically.
</p>

<h3>Windows (PowerShell)</h3>

<pre><code class="language-powershell">irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install.ps1 | iex</code></pre>

<h3>What the Script Does</h3>

<ol>
  <li>Checks for Rust (offers to install it if missing)</li>
  <li>Clones the repository</li>
  <li>Builds the binary with <code>cargo build --release</code></li>
  <li>Installs it to <code>~/.local/bin/</code></li>
  <li>Sets up default config files in <code>~/.config/xfetch/</code></li>
  <li>Adds <code>~/.local/bin</code> to your PATH (via <code>~/.bashrc</code>, <code>~/.zshrc</code>, etc.)</li>
</ol>

<h3>Install Script Options</h3>

<p>
  The install script supports several flags for customization:
</p>

<pre><code class="language-bash"># Install to a custom prefix
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

# Set up config
mkdir -p ~/.config/xfetch
cp configs/config.jsonc ~/.config/xfetch/config.jsonc
cp -r logos/* ~/.config/xfetch/logos/</code></pre>

<hr>

<h2>Install via Cargo</h2>

<pre><code class="language-bash">cargo install --path .</code></pre>

<p>
  This installs to <code>~/.cargo/bin/</code> (ensure it is in your PATH).
</p>

<hr>

<h2>Arch Linux (PKGBUILD)</h2>

<p>
  This method installs xfetch as a proper Arch package, making it easy to update and remove.
</p>

<pre><code class="language-bash">git clone https://github.com/xfetch-cli/xfetch.git
cd xfetch
makepkg -si</code></pre>

<p>
  Installs system-wide to <code>/usr/bin/xfetch</code>.
</p>

<p>
  To uninstall the package:
</p>

<pre><code class="language-bash">sudo pacman -R xfetch-git</code></pre>

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

<h2>Uninstallation</h2>

<p>
  See the <a href="UNINSTALLATION.md">Uninstallation Guide</a> for detailed instructions.
</p>

<p><strong>Quick uninstall:</strong></p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash</code></pre>

<p><strong>Manual removal:</strong></p>

<pre><code class="language-bash">rm -f ~/.local/bin/xfetch
rm -rf ~/.config/xfetch</code></pre>

<p>
  Also remove the PATH line from <code>~/.bashrc</code>, <code>~/.zshrc</code>, or <code>~/.bash_profile</code>:
</p>

<pre><code class="language-bash"># xfetch path
export PATH="$HOME/.local/bin:$PATH"</code></pre>

<hr>

<h2>Next Steps</h2>

<ul>
  <li><a href="CONFIGURATION.md">Configuration Guide</a> — customize modules, logos, colors, and layouts</li>
  <li><a href="LAYOUTS.md">Layouts Guide</a> — explore built-in display layouts</li>
  <li><a href="PLUGINS.md">Plugins Guide</a> — extend xfetch with external plugins</li>
</ul>
