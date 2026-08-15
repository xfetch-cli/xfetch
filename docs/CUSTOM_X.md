<h1>Custom-X Layout</h1>

<p>
  <code>custom-x</code> is the most flexible layout in <code>xfetch</code>: every border
  line is a <strong>literal template</strong> you write in the
  <code>custom_x</code> config object. You decide the characters, the size, the
  internal separators, and even add extra lines — nothing is hardcoded.
</p>

<h2>Enabling</h2>

<pre><code class="language-jsonc">{
    "layout": "custom-x",
    "custom_x": {
        // ... your templates ...
    }
}</code></pre>

<h2>Templates and Placeholders</h2>

<p>All border lines are strings (templates) that support two placeholders:</p>

<ul>
  <li>
    <code>{fill}</code> — repeats the <code>fill</code> character until the line
    reaches the box width. Templates <em>without</em> <code>{fill}</code> are
    extended with the <code>fill</code> character at the end; templates
    <em>longer</em> than the content define the box width.
  </li>
  <li>
    <code>{title}</code> — replaced with the current group title (in
    <code>group_title</code> and <code>top</code> templates).
  </li>
</ul>

<h2>Options</h2>

<table>
  <thead>
    <tr>
      <th>Option</th>
      <th>Default</th>
      <th>Description</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>top</code></td>
      <td><code>╭─ {title}{fill}╮</code></td>
      <td>Top border template. Empty string disables it.</td>
    </tr>
    <tr>
      <td><code>bottom</code></td>
      <td><code>╰{fill}╯</code></td>
      <td>Bottom border template. Empty string disables it.</td>
    </tr>
    <tr>
      <td><code>left</code></td>
      <td><code>│</code></td>
      <td>Prefix for every content row (can be multiple characters).</td>
    </tr>
    <tr>
      <td><code>right</code></td>
      <td><code>│</code></td>
      <td>Suffix for every content row.</td>
    </tr>
    <tr>
      <td><code>fill</code></td>
      <td><code>─</code></td>
      <td>Character used by <code>{fill}</code> and to extend short templates.</td>
    </tr>
    <tr>
      <td><code>padding</code></td>
      <td><code>1</code></td>
      <td>Spaces between the side borders and the content.</td>
    </tr>
    <tr>
      <td><code>width</code></td>
      <td><code>"auto"</code></td>
      <td>
        <code>"auto"</code> sizes the box to the content; <code>"full"</code>
        stretches it to the end of the terminal line (accounting for the logo
        column, minus <code>full_margin</code>); a number sets a fixed width in
        columns.
      </td>
    </tr>
    <tr>
      <td><code>full_margin</code></td>
      <td><code>2</code></td>
      <td>Cells left free at the right edge when <code>width: "full"</code> (avoids the terminal wrap column).</td>
    </tr>
    <tr>
      <td><code>group_title</code></td>
      <td><code>── {title} ──</code></td>
      <td>Template rendered when a module group starts. Empty string hides group titles.</td>
    </tr>
    <tr>
      <td><code>divider</code></td>
      <td><code></code> (off)</td>
      <td>Template rendered as an internal separator. Empty string disables it.</td>
    </tr>
    <tr>
      <td><code>divider_between</code></td>
      <td><code>"groups"</code></td>
      <td>
        Where dividers appear: <code>"groups"</code> (between groups),
        <code>"modules"</code> (also between every module inside groups), or
        <code>"none"</code>.
      </td>
    </tr>
    <tr>
      <td><code>module_top</code></td>
      <td><code></code> (off)</td>
      <td>Template rendered above every module row — wraps each module in its own box.</td>
    </tr>
    <tr>
      <td><code>module_bottom</code></td>
      <td><code></code> (off)</td>
      <td>Template rendered below every module row.</td>
    </tr>
    <tr>
      <td><code>header_lines</code></td>
      <td><code>[]</code></td>
      <td>Extra literal lines rendered after the top border (templates with placeholders allowed).</td>
    </tr>
    <tr>
      <td><code>footer_lines</code></td>
      <td><code>[]</code></td>
      <td>Extra literal lines rendered before the bottom border.</td>
    </tr>
  </tbody>
</table>

<h2>Example</h2>

<p>
  Everything inside one big frame, each group title boxed, and every module
  wrapped in its own box:
</p>

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
        "full_margin": 2,
        "group_title": "╭─── {title} ───{fill}╮",
        "module_top": "╠{fill}╣",
        "module_bottom": "╠{fill}╣",
        "divider_between": "none"
    },
    "modules": [
        { "type": "group", "title": "Hardware", "modules": ["cpu", "gpu"] },
        { "type": "group", "title": "Software", "modules": ["os", "kernel"] }
    ]
}</code></pre>

<p><strong>Result:</strong></p>

<pre><code>╔═══════ XFETCH ═════════════════════════╗
╭─── Hardware ───════════════════════════╮
╠════════════════════════════════════════╣
║   Apple M4 (10) @ 4.46 GHz           ║
╠════════════════════════════════════════╣
║  󰍹 Apple M4                           ║
╠════════════════════════════════════════╣
╭─── Software ───════════════════════════╮
╠════════════════════════════════════════╣
║   Darwin 26.5.2 aarch64              ║
╚════════════════════════════════════════╝</code></pre>

<h2>Section-Style Example (no boxes)</h2>

<p>
  Group headers like <code>hardware──────</code> with <code>────</code>
  separators between groups, no outer frame:
</p>

<pre><code class="language-jsonc">{
    "layout": "custom-x",
    "custom_x": {
        "top": "",
        "bottom": "",
        "left": "",
        "right": "",
        "fill": "─",
        "padding": 0,
        "group_title": "{title}{fill}",
        "divider": "{fill}",
        "divider_between": "groups"
    }
}</code></pre>

<h2>Notes</h2>

<ul>
  <li>Empty templates (<code>""</code>) are skipped entirely — no blank lines are produced.</li>
  <li>With <code>width: "full"</code>, the small-terminal content fallback is applied consistently with the classic layouts.</li>
  <li>Nested groups render recursively (title lines and module rows inside the parent).</li>
  <li>The box width is always at least the widest template/content line; shorter rows are padded inside the right border.</li>
</ul>

<h2>Related</h2>

<ul>
  <li><a href="LAYOUTS.md">xfetch Layouts</a> — built-in layouts</li>
</ul>
