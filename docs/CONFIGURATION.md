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
  <li><code>uptime</code>: System uptime</li>
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
  When <strong>no custom logo is specified</strong>, xfetch uses a built-in default ASCII logo. This logo is rendered with an <strong>orange color</strong> (<code>RGB: 255, 165, 0</code>) applied programmatically.
</p>

<p>
  The color is set in <code>src/ui.rs</code> using CrossTerm:
</p>

<pre><code class="language-rust">SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 })</code></pre>

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

<h3>Images</h3>

<p>
  xfetch supports displaying images (png, jpg, svg) if your terminal supports it (using protocols like iTerm2, Kitty, or Sixel, handled by <code>viuer</code>).
</p>

<pre><code class="language-jsonc">{
    &quot;logo_path&quot;: &quot;/path/to/logo.png&quot;,
    // ...
}</code></pre>

<h2>Logo Animation (Plugin)</h2>

<p>
  xfetch can animate the ASCII logo via an external plugin. The animation runs only on TTY terminals and only for ASCII logos.
</p>

<pre><code class="language-jsonc">{
    &quot;logo_animation&quot;: {
        &quot;plugin&quot;: &quot;animate-logo&quot;,
        &quot;fps&quot;: 12,
        &quot;duration_ms&quot;: 1200,
        &quot;loop&quot;: false
    }
}</code></pre>

<p>
  For plugin installation and the protocol details, see <a href="PLUGINS.md">PLUGINS.md</a>.
</p>

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

<h2>Full Example</h2>

<pre><code class="language-jsonc">{
    &quot;logo_path&quot;: &quot;~/.config/xfetch/logos/ghost.txt&quot;,
    &quot;layout&quot;: &quot;pacman&quot;,
    &quot;header_icons&quot;: [&quot;ᗧ&quot;, &quot;ᗣ&quot;, &quot;ᗣ&quot;],
    &quot;footer_text&quot;: &quot;xfetch&quot;,
    &quot;modules&quot;: [
        &quot;os&quot;,
        &quot;kernel&quot;,
        &quot;cpu&quot;,
        &quot;memory&quot;
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
