<h1>xfetch Layouts</h1>

<p>
  This document explains how to configure and customize layouts in <code>xfetch</code>.
</p>

<h2>Built-in Layouts</h2>

<h3>1. Default Layout (Classic)</h3>

<p>
  The default layout displays the logo (ASCII or image) on the left and the system information modules on the right.
</p>

<p><strong>Configuration:</strong></p>
<p>
  To use this layout, simply omit the <code>layout</code> key in your config or set it to <code>null</code>.
</p>

<pre><code class="language-jsonc">{
    "layout": null,
    // ...
}</code></pre>

<h3>2. Side Block Layout</h3>

<p>
  A structured layout where keys and values are displayed in two separate side-by-side boxes.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "side-block"
}</code></pre>

<p><strong>Appearance:</strong></p>

<pre><code>╭──────────╮ ╭──────────────────────╮
│ User     │ │ jan.rex              │
│ Host     │ │ DIO-LAPTOP           │
╰──────────╯ ╰──────────────────────╯</code></pre>

<h3>3. Tree Layout</h3>

<p>
  Displays modules in a hierarchical tree structure. This layout supports grouping modules.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "tree",
    "modules": [
        {
            "type": "group",
            "title": "OS",
            "modules": ["os", "kernel", "packages"]
        },
        {
            "type": "group",
            "title": "PC",
            "modules": ["cpu", "gpu", "memory"]
        }
    ]
}</code></pre>

<p><strong>Appearance:</strong></p>

<pre><code> OS
├── os Arch Linux
├── kernel 6.6.1
└── packages 1200 (pacman)
 PC
├── cpu AMD Ryzen 9
└── memory 16 GiB</code></pre>

<h3>4. Section Layout</h3>

<p>
  Displays modules in groups with clear section headers.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "section",
    "modules": [
        {
            "type": "group",
            "title": "Hardware",
            "modules": ["cpu", "gpu", "memory"]
        },
        {
            "type": "group",
            "title": "Software",
            "modules": ["os", "shell"]
        }
    ]
}</code></pre>

<p><strong>Appearance:</strong></p>

<pre><code>────── Hardware ──────
│ cpu: AMD Ryzen 9
│ memory: 16 GiB

────── Software ──────
│ os: Arch Linux</code></pre>

<h3>5. Pac-Man Layout</h3>

<p>
  A boxed layout inspired by the Pac-Man game interface.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "pacman",
    "header_icons": ["ᗧ", "●", "●", "●", "●"], // Icons for top border
    "footer_text": "GAME OVER"                 // Text for bottom border
}</code></pre>

<h3>6. Box Layout</h3>

<p>
  Displays the system information enclosed in a simple box with rounded corners.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "box"
}</code></pre>

<h3>7. Line Layout</h3>

<p>
  Displays system information with a horizontal separator line after every 3 modules.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "line"
}</code></pre>

<h3>8. Dots Layout</h3>

<p>
  Similar to the Line layout, but uses dots as separators.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "dots"
}</code></pre>

<h3>9. Bottom Line Layout</h3>

<p>
  A minimal layout that adds a single horizontal line at the very bottom of the information list.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "bottom_line"
}</code></pre>

<h2>10. Section-Box Layout</h2>

<p>
  Like <code>section</code>, but every group is drawn as its own box with a
  title embedded in the top border. Nested groups render as boxes inside boxes.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "section-box",
    "modules": [
        { "type": "group", "title": "Hardware", "modules": ["cpu", "gpu"] },
        { "type": "group", "title": "Software", "modules": ["os", "shell"] }
    ]
}</code></pre>

<p><strong>Appearance:</strong></p>

<pre><code>╭─ Hardware ────────────╮
│  Apple M4           │
│ 󰍹 Apple M4           │
╰───────────────────────╯

╭─ Software ────────────╮
│  Darwin 26.5.2       │
╰───────────────────────╯</code></pre>

<h2>11. Custom-X Layout</h2>

<p>
  The most flexible layout: every border line is a literal template written in
  the <code>custom_x</code> config object, with <code>{fill}</code> and
  <code>{title}</code> placeholders, configurable width, internal dividers and
  per-module boxes. See <a href="CUSTOM_X.md">CUSTOM_X.md</a> for the full
  reference.
</p>

<p><strong>Configuration:</strong></p>

<pre><code class="language-jsonc">{
    "layout": "custom-x",
    "custom_x": {
        "top": "╔═══════ XFETCH ═══════{fill}╗",
        "bottom": "╚══════════════════════{fill}╝",
        "left": "║",
        "right": "║",
        "fill": "═",
        "padding": 1,
        "width": "full",
        "group_title": "╭─── {title} ───{fill}╮"
    },
    "modules": [
        { "type": "group", "title": "Hardware", "modules": ["cpu", "gpu"] }
    ]
}</code></pre>

<h2>Module Grouping</h2>

<p>
  For <code>tree</code> and <code>section</code> layouts, you can define groups in the <code>modules</code> list:
</p>

<pre><code class="language-jsonc">"modules": [
    // Simple module
    "uptime",
    
    // Group
    {
        "type": "group",
        "title": "Group Title",
        "modules": [
            "os",
            "kernel"
            // You can nest groups too!
        ]
    }
]</code></pre>

<h2>Available Modules</h2>

<ul>
  <li><code>os</code>: Operating System</li>
  <li><code>kernel</code>: Kernel version</li>
  <li><code>hostname</code>: Hostname</li>
  <li><code>user</code>: Current username</li>
  <li><code>datetime</code>: Current date and time</li>
  <li><code>uptime</code>: System uptime</li>
  <li><code>packages</code>: Package count</li>
  <li><code>shell</code>: Current shell</li>
  <li><code>terminal</code>: Current terminal emulator</li>
  <li><code>wm</code>: Window Manager / Desktop Environment</li>
  <li><code>cpu</code>: CPU information</li>
  <li><code>gpu</code>: GPU information</li>
  <li><code>memory</code>: RAM usage</li>
  <li><code>swap</code>: Swap usage</li>
  <li><code>disk</code>: Disk usage</li>
  <li><code>battery</code>: Battery status</li>
  <li><code>local_ip</code>: Local IP address</li>
  <li><code>palette</code>: Color palette</li>
</ul>

<h2>Icons</h2>

<p>
  We recommend using <a href="https://www.nerdfonts.com/">Nerd Fonts</a> for icons to ensure they render correctly. You can customize icons in the <code>icons</code> section of your config file.
</p>

<pre><code class="language-jsonc">"icons": {
    "os": "",
    "cpu": "",
    // ...
}</code></pre>
