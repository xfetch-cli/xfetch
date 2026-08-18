<h1>Configuration Guide</h1>

<p>
  <strong>xfetch</strong> is highly customizable using JSONC (JSON with Comments) files. This guide explains how to customize every aspect of the tool.
</p>

<h2>Config File Location</h2>

<p>
  By default, xfetch looks for a configuration file at:
</p>

<ul>
  <li><strong>Linux</strong>: <code>~/.config/xfetch/config.jsonc</code></li>
  <li><strong>Windows</strong>: <code>%APPDATA%\xfetch\config.jsonc</code></li>
  <li><strong>macOS</strong>: <code>~/Library/Application Support/xfetch/config.jsonc</code></li>
</ul>

<p>
  You can also pass a custom config file using the <code>--config</code> flag:
</p>

<pre><code class="language-bash">xfetch --config path/to/my_config.jsonc</code></pre>

<p>
  Curated presets and plugin-oriented examples are maintained separately in
  <a href="https://github.com/xfetch-cli/configs">xfetch-cli/configs</a>.
</p>

<h2>Command Line Options</h2>

<table>
  <thead>
    <tr><th>Flag / Command</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>xfetch</code></td><td>Render the fetch with the default (or configured) layout.</td></tr>
    <tr><td><code>xfetch --config &lt;path&gt;</code></td><td>Use a custom config file.</td></tr>
    <tr><td><code>xfetch --daemon</code></td><td>Run in daemon mode: pin an animated fetch at the top of the terminal while keeping the prompt usable below. See <a href="DAEMON.md">DAEMON.md</a>.</td></tr>
    <tr><td><code>xfetch --daemon-stop</code></td><td>Stop the running daemon.</td></tr>
    <tr><td><code>xfetch --gen-config</code></td><td>Generate a starter config at the default location.</td></tr>
    <tr><td><code>xfetch --clean-cache</code></td><td>Clear the module cache.</td></tr>
    <tr><td><code>xfetch --benchmark</code></td><td>Print benchmarking info for the info gathering step.</td></tr>
    <tr><td><code>xfetch plugin install|list|remove &lt;name&gt;</code></td><td>Manage plugins. See <a href="PLUGINS.md">PLUGINS.md</a>.</td></tr>
    <tr><td><code>xfetch extension install|list|remove &lt;name&gt;</code></td><td>Manage extensions. See <a href="EXTENSIONS.md">EXTENSIONS.md</a>.</td></tr>
    <tr><td><code>xfetch theme list|set|remove|export &lt;name&gt;</code></td><td>Manage themes. See <a href="THEMES.md">THEMES.md</a>.</td></tr>
  </tbody>
</table>

<h2>Themes</h2>

<p>
  xfetch has a theme system that lets you switch visual styles (colors, icons, layout) independently from your module configuration.
</p>

<p>
  Set the active theme in your config with the <code>theme</code> field:
</p>

<pre><code class="language-jsonc">{
    &quot;theme&quot;: &quot;berlin&quot;,
    &quot;modules&quot;: [&quot;os&quot;, &quot;kernel&quot;, &quot;memory&quot;]
}</code></pre>

<p>
  Theme files live in <code>~/.config/xfetch/themes/&lt;name&gt;.jsonc</code> and contain only visual fields (colors, icons, layout). The theme merges with your config with the theme having highest priority.
</p>

<p>See <a href="THEMES.md">THEMES.md</a> for the full theme system reference.</p>

<h2>Basic Structure</h2>

<p>
  A minimal configuration looks like this:
</p>

<pre><code class="language-jsonc">{
    &quot;modules&quot;: [&quot;os&quot;, &quot;kernel&quot;, &quot;memory&quot;],
    &quot;show_colors&quot;: true
}</code></pre>

<h2>Customizing Modules</h2>

<p>
  The <code>modules</code> array determines which information is displayed and in what order.
</p>

<p><strong>Available Modules:</strong></p>

<ul>
  <li><code>os</code>: Operating System name and architecture</li>
  <li><code>kernel</code>: Kernel version</li>
  <li><code>hostname</code>: Hostname of the machine</li>
  <li><code>user</code>: Current username</li>
  <li><code>uptime</code>: System uptime</li>
  <li><code>datetime</code>: Current date and time</li>
  <li><code>packages</code>: Package count (pacman, dpkg, brew, scoop, etc.)</li>
  <li><code>shell</code>: Current shell (bash, zsh, powershell, etc.)</li>
  <li><code>terminal</code>: Current terminal emulator</li>
  <li><code>wm</code>: Window Manager / Desktop Environment</li>
  <li><code>cpu</code>: CPU model and frequency</li>
  <li><code>gpu</code>: GPU model</li>
  <li><code>memory</code>: RAM usage</li>
  <li><code>swap</code>: Swap memory usage</li>
  <li><code>disk</code>: Disk usage (first disk)</li>
  <li><code>battery</code>: Battery percentage and status</li>
  <li><code>local_ip</code>: Local IPv4 address</li>
  <li><code>local_ip:v6</code>: Local IPv6 address</li>
  <li><code>public_ip</code>: Public IP address (requires network access)</li>
  <li><code>interfaces</code>: Network interfaces</li>
  <li><code>palette</code>: Color palette</li>
</ul>

<h2>Logos and ASCII Art</h2>

<p>
  You can display custom logos using text files or images.
</p>

<h3>Color System for ASCII Logos</h3>

<p>
  xfetch supports two methods for coloring ASCII logos:
</p>

<h4>1. ANSI Escape Codes in Custom ASCII Files</h4>

<p>
  When using a custom ASCII logo file (via <code>logo_path</code> or <code>ascii</code>), you can embed <strong>ANSI escape codes</strong> directly in the text file to add colors. The escape codes are interpreted by the terminal to render colored text.
</p>

<p><strong>Format:</strong> <code>\x1b[&lt;code&gt;m</code> or <code>\033[&lt;code&gt;m</code></p>

<p><strong>Available Foreground Color Codes:</strong></p>

<table>
  <thead>
    <tr><th>Color</th><th>Code</th><th>Example</th></tr>
  </thead>
  <tbody>
    <tr><td>Black</td><td>30</td><td><code>\x1b[30mText\x1b[0m</code></td></tr>
    <tr><td>Red</td><td>31</td><td><code>\x1b[31mText\x1b[0m</code></td></tr>
    <tr><td>Green</td><td>32</td><td><code>\x1b[32mText\x1b[0m</code></td></tr>
    <tr><td>Yellow</td><td>33</td><td><code>\x1b[33mText\x1b[0m</code></td></tr>
    <tr><td>Blue</td><td>34</td><td><code>\x1b[34mText\x1b[0m</code></td></tr>
    <tr><td>Magenta</td><td>35</td><td><code>\x1b[35mText\x1b[0m</code></td></tr>
    <tr><td>Cyan</td><td>36</td><td><code>\x1b[36mText\x1b[0m</code></td></tr>
    <tr><td>White</td><td>37</td><td><code>\x1b[37mText\x1b[0m</code></td></tr>
    <tr><td>Gray</td><td>90</td><td><code>\x1b[90mText\x1b[0m</code></td></tr>
  </tbody>
</table>

<p><strong>256-Color Mode:</strong> <code>\x1b[38;5;&lt;n&gt;m</code> where &lt;n&gt; is 0-255</p>

<p><strong>RGB True Color:</strong> <code>\x1b[38;2;&lt;r&gt;;&lt;g&gt;;&lt;b&gt;m</code></p>

<p><strong>Reset Code:</strong> <code>\x1b[0m</code> (resets all formatting)</p>

<p><strong>Example ASCII Logo with Colors (<code>x_logo.txt</code>):</strong></p>

<pre><code class="language-plain">\x1b[36m      \\\\\\      ///
\x1b[36m       \\\\\\    ///
\x1b[35m        \\\\\\  ///
\x1b[35m         \\\\///
\x1b[33m         ///\\\\
\x1b[33m        ///  \\\\\\
\x1b[32m       ///    \\\\\\
\x1b[32m      ///      \\\\\</code></pre>

<p>This creates a gradient effect from cyan to green.</p>

<h4>2. Default ASCII Logo Color</h4>

<p>
  When <strong>no custom logo is specified</strong>, xfetch uses a built-in default ASCII logo. This logo is rendered with a <strong>gray color</strong> (<code>RGB: 128, 128, 128</code>) applied programmatically.
</p>

<p>
  The color is set in <code>src/ui/print.rs</code> (<code>DEFAULT_LOGO_COLOR</code>) using crossterm:
</p>

<pre><code class="language-rust">SetForegroundColor(Color::Rgb { r: 128, g: 128, b: 128 })</code></pre>

<blockquote><strong>Note:</strong> Custom ASCII logos bypass this automatic coloring and use their embedded ANSI codes instead.</blockquote>

<h3>Text/ASCII Logos</h3>

<p>
  Create a text file (e.g., <code>logo.txt</code>). You can use ANSI escape codes for colors in this file.
</p>

<pre><code class="language-jsonc">{
    // You can use tilde (~) for home directory
    &quot;logo_path&quot;: &quot;~/.config/xfetch/logos/arch.txt&quot;,
    // ...
}</code></pre>

<h3>ASCII Logo Options</h3>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>logo_color</code></td><td>string</td><td>none</td>
      <td>Color applied to the ASCII logo. Accepts names (<code>&quot;Cyan&quot;</code>), 256-color indexes (<code>&quot;196&quot;</code>) and hex RGB (<code>&quot;#FF0000&quot;</code>). Also applies to animated logos; lines that already contain ANSI codes are left untouched.</td>
    </tr>
    <tr>
      <td><code>logo_padding</code></td><td>number</td><td>0</td>
      <td>Leading spaces added before the logo (and its frames when animated).</td>
    </tr>
    <tr>
      <td><code>logo_type</code></td><td>string</td><td><code>&quot;auto&quot;</code></td>
      <td><code>&quot;auto&quot;</code> detects by file extension, <code>&quot;ascii&quot;</code> forces text rendering, <code>&quot;image&quot;</code> forces image rendering.</td>
    </tr>
  </tbody>
</table>

<pre><code class="language-jsonc">{
    &quot;ascii&quot;: &quot;~/.config/xfetch/logos/arch.txt&quot;,
    &quot;logo_color&quot;: &quot;#00FF87&quot;,
    &quot;logo_padding&quot;: 2
}</code></pre>

<h3>Images</h3>

<p>
  xfetch supports displaying images (png, jpg, svg) if your terminal supports it (using protocols like iTerm2, Kitty, or Sixel, handled by <code>viuer</code>).
</p>

<pre><code class="language-jsonc">{
    &quot;logo_path&quot;: &quot;/path/to/logo.png&quot;,
    // ...
}</code></pre>

<h4>Image Logo Options</h4>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>logo_width</code></td><td>number</td><td>auto (28% of terminal width, clamped 12-42)</td>
      <td>Explicit image width in terminal columns.</td>
    </tr>
    <tr>
      <td><code>logo_height</code></td><td>number</td><td>auto</td>
      <td>Explicit image height in terminal rows.</td>
    </tr>
    <tr>
      <td><code>logo_gap</code></td><td>number</td><td>12</td>
      <td>Gap (in columns) between the logo and the module text.</td>
    </tr>
    <tr>
      <td><code>logo_kitty</code></td><td>boolean</td><td><code>true</code></td>
      <td>Use the native Kitty graphics protocol when running inside Kitty; set to <code>false</code> to fall back to half-block rendering.</td>
    </tr>
  </tbody>
</table>

<h2>Logo Animation (Plugin)</h2>

<p>
  xfetch can animate the ASCII logo via an external plugin. The animation runs only on TTY terminals and only for ASCII logos (not images).
</p>

<pre><code class="language-jsonc">{
    &quot;logo_animation&quot;: {
        &quot;plugin&quot;: &quot;animate-logo&quot;,
        &quot;style&quot;: &quot;frame&quot;,
        &quot;fps&quot;: 6,
        &quot;duration_ms&quot;: 8000,
        &quot;loop&quot;: true,
        &quot;frames_path&quot;: &quot;~/.config/xfetch/logos/fox.txt&quot;
    }
}</code></pre>

<table>
  <thead>
    <tr><th>Field</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>plugin</code></td><td>Plugin short name or full path to the executable.</td></tr>
    <tr><td><code>style</code></td><td>Animation style: <code>sweep</code> (default), <code>wave</code>, <code>rainbow</code>, <code>sparkle</code>, <code>breathing</code>, <code>frame</code>, or <code>none</code>.</td></tr>
    <tr><td><code>fps</code></td><td>Frames per second (speed of the animation).</td></tr>
    <tr><td><code>duration_ms</code></td><td>Total animation duration in milliseconds. Only honored outside daemon mode.</td></tr>
    <tr><td><code>loop</code></td><td>Loop the animation (<code>true</code>/<code>false</code>). Only honored outside daemon mode.</td></tr>
    <tr><td><code>frames_path</code></td><td>Frame source for the <code>frame</code> style: a single file with frames separated by a line containing <code>===</code>, or an array of files (one per frame).</td></tr>
  </tbody>
</table>

<blockquote>
  <strong>Note:</strong> With <code>"daemon": true</code> the animation runs as an infinite loop pinned at the top of the terminal, and <code>duration_ms</code>/<code>loop</code> are ignored. To play a finite animation that stops on its own, leave daemon mode off. See <a href="DAEMON.md">DAEMON.md</a>.
</blockquote>

<p>
  For plugin installation and the protocol details, see <a href="PLUGINS.md">PLUGINS.md</a>.
</p>

<h2>Daemon Mode</h2>

<p>
  Daemon mode pins an animated fetch at the top of the terminal and keeps looping it in the background without blocking the shell prompt.
</p>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>daemon</code></td><td>boolean</td><td><code>false</code></td>
      <td>Enable daemon mode. Equivalent to the <code>--daemon</code> CLI flag (the CLI flag overrides the config value).</td>
    </tr>
    <tr>
      <td><code>daemon_min_rows</code></td><td>number</td><td>6</td>
      <td>Minimum number of terminal rows left free below the pinned block for command output.</td>
    </tr>
  </tbody>
</table>

<p>
  Full reference, usage and troubleshooting: see <a href="DAEMON.md">DAEMON.md</a>.
</p>

<h2>Config Providers (Extensions)</h2>

<p>
  Extensions run at config load time and can modify or replace the effective config. They are declared with <code>config_providers</code>:
</p>

<pre><code class="language-jsonc">{
    &quot;config_providers&quot;: [
        {
            &quot;extension&quot;: &quot;config-roulette&quot;,
            &quot;args&quot;: {
                &quot;routes&quot;: &quot;~/.config/xfetch/routes.json&quot;,
                &quot;strategy&quot;: &quot;random&quot;
            }
        }
    ]
}</code></pre>

<p>
  Full reference: see <a href="EXTENSIONS.md">EXTENSIONS.md</a>.
</p>

<h2>Info Plugins</h2>

<p>
  Plugins can contribute extra module lines. They are declared with <code>info_plugins</code>, each with a <code>plugin</code> name/path and optional <code>args</code>:
</p>

<pre><code class="language-jsonc">{
    &quot;info_plugins&quot;: [
        {
            &quot;plugin&quot;: &quot;weather&quot;,
            &quot;args&quot;: { &quot;city&quot;: &quot;Madrid&quot; }
        }
    ]
}</code></pre>

<h2>Palette Style</h2>

<p>
  The <code>palette</code> module can render in different styles via <code>palette_style</code>:
</p>

<ul>
  <li><code>squares</code> (default)</li>
  <li><code>circles</code></li>
  <li><code>triangles</code></li>
  <li><code>lines</code></li>
</ul>

<pre><code class="language-jsonc">{
    &quot;palette_style&quot;: &quot;circles&quot;
}</code></pre>

<h2>Privacy &amp; Cache</h2>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>disable_ip_fetching</code></td><td>boolean</td>
      <td>Skip the network request for the <code>public_ip</code> module.</td>
    </tr>
    <tr>
      <td><code>disable_cache</code></td><td>boolean</td>
      <td>Disable the on-disk cache used for slow probes (e.g. package counts).</td>
    </tr>
    <tr>
      <td><code>os_wsl_style</code></td><td>string</td>
      <td>WSL OS presentation (Linux only): <code>off</code> (plain name), <code>minimal</code> (appends <code>(WSL)</code>), <code>full</code> (appends WSL version and WSLg). Default: <code>minimal</code>.</td>
    </tr>
  </tbody>
</table>

<h2>Layouts</h2>

<h3>Default Layout</h3>

<p>
  The standard &quot;side-by-side&quot; fetch layout.
</p>

<pre><code class="language-jsonc">{
    &quot;layout&quot;: null // or omit this field
}</code></pre>

<h3>Pac-Man Layout</h3>

<p>
  A boxed layout with a custom header and footer, inspired by Pac-Man.
</p>

<pre><code class="language-jsonc">{
    &quot;layout&quot;: &quot;pacman&quot;,
    // Icons displayed in the top border
    &quot;header_icons&quot;: [&quot;ᗧ&quot;, &quot;●&quot;, &quot;●&quot;, &quot;●&quot;],
    // Text displayed in the bottom border
    &quot;footer_text&quot;: &quot;GAME OVER&quot;
}</code></pre>

<h2>Icons and Emojis</h2>

<p>
  You can customize the icon displayed next to each module. You can use standard Emojis or Nerd Fonts.
</p>

<pre><code class="language-jsonc">{
    &quot;icons&quot;: {
        &quot;os&quot;: &quot;&quot;,      // Arch Linux icon (Nerd Font)
        &quot;cpu&quot;: &quot;🧠&quot;,    // Brain emoji
        &quot;memory&quot;: &quot;RAM:&quot; // Plain text
    }
}</code></pre>

<h2>Colors</h2>

<p>
  You can set the color for the icon/label of each module.
</p>

<p><strong>Available Colors:</strong></p>

<ul>
  <li><code>Black</code></li>
  <li><code>Red</code></li>
  <li><code>Green</code></li>
  <li><code>Yellow</code></li>
  <li><code>Blue</code></li>
  <li><code>Magenta</code></li>
  <li><code>Cyan</code></li>
  <li><code>White</code></li>
  <li><code>Grey</code> (or <code>Gray</code>)</li>
  <li><code>DarkGrey</code> (or <code>DarkGray</code>)</li>
  <li><code>DarkRed</code></li>
  <li><code>DarkGreen</code></li>
  <li><code>DarkYellow</code></li>
  <li><code>DarkBlue</code></li>
  <li><code>DarkMagenta</code></li>
  <li><code>DarkCyan</code></li>
</ul>

<pre><code class="language-jsonc">{
    &quot;colors&quot;: {
        &quot;os&quot;: &quot;Cyan&quot;,
        &quot;cpu&quot;: &quot;Red&quot;,
        &quot;memory&quot;: &quot;Green&quot;
    }
}</code></pre>

<h2>Keys (Labels)</h2>

<p>
  By default xfetch renders each module as <code>icon value</code>. To display the module label as well, enable <code>show_keys</code>; use <code>key_width</code> to pad the labels to a fixed column count so values align vertically.
</p>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>show_keys</code></td><td>boolean</td><td><code>false</code></td>
      <td>Render <code>key: value</code> in the icon-style layouts (classic, section, compact, custom-x, box variants).</td>
    </tr>
    <tr>
      <td><code>key_width</code></td><td>number</td><td>auto</td>
      <td>Pad the key to this many columns before the <code>:</code> separator. Applies wherever keys are shown, including <code>section</code> and <code>minimal</code>.</td>
    </tr>
  </tbody>
</table>

<pre><code class="language-jsonc">{
    &quot;show_keys&quot;: true,
    &quot;key_width&quot;: 12
}</code></pre>

<h2>Full Example</h2>

<pre><code class="language-jsonc">{
    &quot;ascii&quot;: &quot;~/.config/xfetch/logos/ghost.txt&quot;,
    &quot;logo_gap&quot;: 8,
    &quot;logo_color&quot;: &quot;Cyan&quot;,
    &quot;show_keys&quot;: true,
    &quot;key_width&quot;: 12,
    &quot;theme&quot;: &quot;berlin&quot;,
    &quot;layout&quot;: &quot;pacman&quot;,
    &quot;header_icons&quot;: [&quot;ᗧ&quot;, &quot;ᗣ&quot;, &quot;ᗣ&quot;],
    &quot;footer_text&quot;: &quot;xfetch&quot;,
    &quot;palette_style&quot;: &quot;circles&quot;,
    &quot;logo_animation&quot;: {
        &quot;plugin&quot;: &quot;animate-logo&quot;,
        &quot;style&quot;: &quot;sweep&quot;,
        &quot;fps&quot;: 12,
        &quot;duration_ms&quot;: 1200,
        &quot;loop&quot;: false
    },
    &quot;modules&quot;: [
        &quot;os&quot;,
        &quot;kernel&quot;,
        &quot;cpu&quot;,
        &quot;memory&quot;,
        &quot;palette&quot;
    ],
    &quot;show_colors&quot;: true,
    &quot;icons&quot;: {
        &quot;os&quot;: &quot;&quot;,
        &quot;cpu&quot;: &quot;&quot;,
        &quot;memory&quot;: &quot;&quot;
    },
    &quot;colors&quot;: {
        &quot;os&quot;: &quot;Blue&quot;,
        &quot;cpu&quot;: &quot;Red&quot;,
        &quot;memory&quot;: &quot;Yellow&quot;
    }
}</code></pre>
