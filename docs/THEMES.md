<h1>Themes Guide</h1>

<p>
  xfetch has a theme system that separates visual appearance (colors, icons, layout) from module configuration, letting you switch looks without touching your module setup.
</p>

<h2>Architecture</h2>

<p>
  Themes operate at the config-loading layer, before any rendering or plugin execution. When you set <code>"theme": "berlin"</code> in <code>config.jsonc</code>, xfetch resolves the theme file, parses it, and merges three layers together:
</p>

<pre><code>config.jsonc (theme: "berlin", modules: [...])
       |
       v
   1. Load default Config (hardcoded defaults)
       |
       v
   2. Load user config.jsonc
       |
       v
   3. Load theme/berlin.jsonc  ← highest priority
       |
       v
   4. Deep-merge: defaults → config.jsonc → theme
       |
       v
   Final Config (used by renderer)
</code></pre>

<h2>Merge Order (last wins)</h2>

<p>Starting from xfetch v0.2.0, the merge order gives the theme the highest priority:</p>

<table>
  <thead>
    <tr><th>Layer</th><th>Source</th><th>Contains</th></tr>
  </thead>
  <tbody>
    <tr><td>1. Core defaults</td><td>Hardcoded in <code>config.rs</code></td><td>Default icons, colors, layout, modules</td></tr>
    <tr><td>2. User config</td><td><code>config.jsonc</code></td><td><code>modules</code>, <code>info_plugins</code>, plus any visual overrides</td></tr>
    <tr><td>3. Theme file</td><td><code>themes/&lt;name&gt;.jsonc</code></td><td><code>colors</code>, <code>logo_color</code>, <code>layout</code>, <code>palette_style</code>, <code>show_colors</code>, <code>header_icons</code>, <code>footer_text</code>, <code>logo_path</code> (no <code>icons</code> — they belong to the user config)</td></tr>
  </tbody>
</table>

<p>
  A field in the theme file <strong>always wins</strong> over the same field in <code>config.jsonc</code> or defaults. This means the theme provides the final look, while <code>config.jsonc</code> handles module selection and plugin config.
</p>

<p>
  <strong>Note:</strong> In v0.1.x the order was reversed (config.jsonc won over theme). If you're upgrading, you may need to move any visual overrides from <code>config.jsonc</code> into the theme file if you want the theme to control them.
</p>

<h3>Merge details</h3>

<p>The merge uses <code>deep_merge()</code> which works key-by-key:</p>

<ul>
  <li><strong>Objects:</strong> merged recursively — each key is resolved independently</li>
  <li><strong>Strings, numbers, booleans:</strong> overlay replaces base</li>
  <li><strong>Empty strings:</strong> an empty string <em>does not</em> override a non-empty string (prevents themes from accidentally clearing icons)</li>
  <li><strong>Empty objects <code>{}</code>:</strong> no-op — existing keys are preserved</li>
</ul>

<h2>Theme File Format</h2>

<p>A theme file is a JSONC document containing only visual fields:</p>

<pre><code class="language-jsonc">{
    "layout": "section",
    "show_colors": true,
    "palette_style": "circles",
    "logo_color": "Magenta",
    "colors": {
        "os": "Magenta",
        "cpu": "Red",
        "memory": "Yellow",
        "disk": "Cyan",
        "shell": "Green",
        "wm": "Blue"
    }
}
</code></pre>

<p>
  Themes declare only what they want to change; everything else (icons,
  modules, fonts) comes from the user's config and the defaults. <strong>Icons
  are not part of themes</strong> — they are a per-user font choice, so
  exported and registry themes never include an <code>icons</code> block.
  <code>logo_color</code> colors the ASCII logo (any color name listed
  below).
</p>

<h3>Supported Fields</h3>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>layout</code></td><td><code>string</code> or <code>null</code></td><td>Layout style: <code>default</code>, <code>pacman</code>, <code>section</code>, <code>side-block</code>, <code>tree</code>, <code>compact</code>, <code>minimal</code>, <code>horizontal</code>, <code>bottom</code>, <code>box</code>, <code>line</code>, <code>dots</code>, <code>bottom-line</code></td></tr>
    <tr><td><code>colors</code></td><td><code>object</code></td><td>Per-module color mapping (labels + icons). Keys are module names, values are color names: <code>Black</code>, <code>Red</code>, <code>Green</code>, <code>Yellow</code>, <code>Blue</code>, <code>Magenta</code>, <code>Cyan</code>, <code>White</code>, <code>Grey</code>/<code>Gray</code>, and dark variants. Any module key works, including <code>plugin:&lt;name&gt;</code> entries.</td></tr>
    <tr><td><code>logo_color</code></td><td><code>string</code> or <code>null</code></td><td>Color for the ASCII logo (any color name listed above).</td></tr>
    <tr><td><code>logo_colors</code></td><td><code>array</code> or <code>null</code></td><td>Per-row logo colors: row <code>i</code> uses <code>logo_colors[i % len]</code>, cycling for taller logos. Takes precedence over <code>logo_color</code>. Example: <code>["Red", "Yellow", "Green", "Cyan", "Blue", "Magenta"]</code>.</td></tr>
    <tr><td><code>palette_style</code></td><td><code>string</code> or <code>null</code></td><td>Palette display: <code>squares</code>, <code>circles</code>, <code>triangles</code>, <code>lines</code>, <code>dots</code></td></tr>
    <tr><td><code>show_colors</code></td><td><code>boolean</code></td><td>Show inline ANSI color swatches next to each module</td></tr>
    <tr><td><code>logo_path</code></td><td><code>string</code> or <code>null</code></td><td>Path to a logo file (ASCII, PNG, SVG)</td></tr>
    <tr><td><code>header_icons</code></td><td><code>array</code> or <code>null</code></td><td>Pac-Man layout top-border icons</td></tr>
    <tr><td><code>footer_text</code></td><td><code>string</code> or <code>null</code></td><td>Pac-Man layout bottom-border text</td></tr>
  </tbody>
</table>

<h2>Theme Resolution</h2>

<p>
  When xfetch loads a config with <code>"theme": "dracula"</code>, it searches in order:
</p>

<ol>
  <li><strong>Direct path</strong> — If the value contains <code>/</code> or starts with <code>~</code>, load it directly as a file path</li>
  <li><strong>Themes directory</strong> — Look for <code>&lt;name&gt;.jsonc</code> in <code>~/.config/xfetch/themes/</code></li>
</ol>

<p>If the theme is not found, the config loads without the theme (no error).</p>

<h2>CLI Commands</h2>

<pre><code class="language-bash"># List installed themes
xfetch theme list

# Activate a theme (writes "theme" field into config.jsonc)
xfetch theme set nord

# Remove a theme file
xfetch theme remove nord

# Export current visual config as a reusable theme file
xfetch theme export my-theme
</code></pre>

<h3>Export Behavior</h3>

<p>
  <code>xfetch theme export &lt;name&gt;</code> captures the current runtime visual state (after all three layers are merged) and writes it to <code>~/.config/xfetch/themes/&lt;name&gt;.jsonc</code>. This lets you share your look without exposing your module list or plugin config.
</p>

<p>The exported file contains only visual fields: <code>layout</code>, <code>colors</code>, <code>icons</code>, <code>palette_style</code>, <code>header_icons</code>, <code>footer_text</code>, <code>logo_path</code>, <code>show_colors</code>.</p>

<h2>Directory Structure</h2>

<pre><code>~/.config/xfetch/
    config.jsonc            # Modules, plugins, and theme reference
    themes/
        berlin.jsonc        # Theme files: colors, icons, layout
        dracula.jsonc
        nord.jsonc
        catppuccin-mocha.jsonc
        retro-pacman.jsonc
        tree-compact.jsonc
</code></pre>

<h2>Migrating from v0.1.x</h2>

<p>
  In v0.1.x, <code>config.jsonc</code> won over the theme. If you had a detailed config with explicit colors/icons and then set a theme, nothing would visibly change.
</p>

<p>To migrate to v0.2.0+:</p>

<ol>
  <li>Export your current look: <code>xfetch theme export my-look</code></li>
  <li>Set it: <code>xfetch theme set my-look</code></li>
  <li>Remove visual fields (<code>colors</code>, <code>icons</code>, <code>layout</code>, etc.) from <code>config.jsonc</code> so the theme can fully control them</li>
  <li>Keep <code>modules</code> and <code>info_plugins</code> in <code>config.jsonc</code></li>
</ol>

<h2>Best Practices</h2>

<ul>
  <li>Keep visual fields (colors, icons, layout) in theme files, not in <code>config.jsonc</code></li>
  <li>Keep structural fields (modules, info_plugins) in <code>config.jsonc</code></li>
  <li>Create a separate theme file for each visual variant you want</li>
  <li>Export your current config as a starting point: <code>xfetch theme export my-base</code></li>
</ul>
