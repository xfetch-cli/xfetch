<h1>xfetch Config Generation (<code>--gen-config</code>)</h1>

<p>
  <code>xfetch --gen-config</code> generates a starter config file so you can run and customize
  xfetch without hand-writing one. It writes a ready-to-use template and, when network is
  available, attaches the ASCII logo of your OS/distro.
</p>

<h2>Basic Usage</h2>

<p>
  Generate the config at the default location (<code>~/.config/xfetch/config.jsonc</code> on
  Linux, <code>~/Library/Application Support/xfetch/config.jsonc</code> on macOS,
  <code>%APPDATA%\xfetch\config.jsonc</code> on Windows):
</p>

<pre><code>xfetch --gen-config</code></pre>

<p>Generate it somewhere else:</p>

<pre><code>xfetch --gen-config --config ~/my-setup/xfetch-config.jsonc</code></pre>

<h2>Distro Logo (<code>--logo</code>)</h2>

<p>
  By default the generated config includes the ASCII logo of the detected OS/distro, fetched
  from the <a href="https://github.com/xfetch-cli/logos">xfetch-cli/logos</a> catalog.
</p>

<h3>Automatic Detection</h3>

<p>
  The detection reads <code>/etc/os-release</code> (<code>ID</code> + <code>ID_LIKE</code>) on
  Linux, and maps the OS version on macOS (<code>macos-ventura</code>, ...) and Windows
  (<code>windows-11</code>, ...). Resolution order: exact <code>ID</code> → each
  <code>ID_LIKE</code> token → generic logo of the category.
</p>

<p>
  The fetched art is stored in <code>&lt;config_dir&gt;/xfetch/logos/&lt;distro-id&gt;.txt</code>
  and the generated config references it via the <code>ascii</code> key:
</p>

<pre><code class="language-jsonc">{
    "ascii": "/home/jan/.config/xfetch/logos/ubuntu.txt",
    "layout": "pacman",
    // ...
}</code></pre>

<h3>Logo Override</h3>

<p>
  Choose a specific logo with <code>--logo</code>, overriding the detected OS/distro. Any
  catalog id or alias works, including logos of other OSes:
</p>

<pre><code># Ubuntu machine, Arch logo
xfetch --gen-config --logo arch

# Force a specific OS/version logo
xfetch --gen-config --logo windows-11
xfetch --gen-config --logo macos-ventura</code></pre>

<h3>Fallbacks</h3>

<ul>
  <li>
    <strong>Unknown <code>--logo</code> id:</strong> warns and uses the generic logo of the
    current category, saved as <code>default.txt</code>.
  </li>
  <li>
    <strong>No network / catalog error / invalid art:</strong> the template is written without
    a logo — the previous behavior. Automatic detection falls back silently; an explicit
    <code>--logo</code> prints a warning.
  </li>
  <li>
    <strong>Offline cache:</strong> once a logo is downloaded it stays in
    <code>&lt;config_dir&gt;/xfetch/logos/</code>, so regenerating later references the same
    file even without internet.
  </li>
</ul>

<p>
  The catalog base URL can be overridden for testing forks or mirrors:
</p>

<pre><code>XFETCH_LOGOS_URL=https://raw.githubusercontent.com/&lt;user&gt;/logos/main \
    xfetch --gen-config --logo arch</code></pre>

<h2>Layout (<code>--layout</code>)</h2>

<p>
  The template ships with the <code>section</code> layout (grouped Hardware/Software/Session
  modules). Use <code>--layout</code> to generate it with any of the built-in layouts (see
  <a href="LAYOUTS.md">LAYOUTS.md</a>):
</p>

<pre><code>xfetch --gen-config --layout pacman
xfetch --gen-config --layout tree
xfetch --gen-config --layout compact</code></pre>

<p>Available layout names:</p>

<ul>
  <li><code>default</code>, <code>side-block</code>, <code>tree</code>, <code>section</code>, <code>section-box</code></li>
  <li><code>custom-x</code>, <code>compact</code>, <code>minimal</code></li>
  <li><code>pacman</code>, <code>box</code>, <code>line</code>, <code>dots</code>, <code>bottom_line</code> (classic variants)</li>
  <li><code>horizontal</code>, <code>bottom</code> (vertical layouts)</li>
</ul>

<p>An unknown layout name warns and keeps <code>pacman</code>.</p>

<h2>Combined Example</h2>

<p>Generate a tree-layout config with the Arch logo on an Ubuntu machine:</p>

<pre><code>xfetch --gen-config --layout tree --logo arch --config ~/configs/arch-tree.jsonc</code></pre>

<p>
  The <code>--logo</code> and <code>--layout</code> flags only apply to
  <code>--gen-config</code>; runtime behavior is untouched.
</p>
