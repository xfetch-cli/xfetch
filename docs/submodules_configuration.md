<h1>Submodule Configuration</h1>

<p>
  Every value xfetch renders is a <em>submodule</em>: a module key (e.g. <code>cpu</code>) that produces a value, an icon and an optional row key. Two config maps give you full control over how submodules look, without touching the probe logic:
</p>

<ul>
  <li><code>labels</code> — rename or hide the row key shown per module.</li>
  <li><code>formats</code> — replace a module's value with a template of <code>{field}</code> placeholders.</li>
</ul>

<p>
  Both are optional and additive: an empty map (or absent key) keeps the default output byte-for-byte, so existing configs keep working unchanged. Formatting is applied once, when the render tree is built, so it works identically in <strong>every layout</strong> (classic and variants, compact, minimal, section, section-box, tree, custom-x) and in both daemons (animated-logo and live stats).
</p>

<h2><code>labels</code> — Row Keys</h2>

<p>
  The <code>labels</code> map renames the key shown for a module. An empty string hides the key entirely, leaving an icon-only row. Modules without an entry keep their module name.
</p>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>labels</code></td><td>object</td><td><code>{}</code></td>
      <td>Per-module key labels: <code>&quot;cpu&quot;: &quot;procesador&quot;</code> renames the key; <code>&quot;gpu&quot;: &quot;&quot;</code> hides it. Colors keep using the raw module key, so renaming never breaks the color mapping.</td>
    </tr>
  </tbody>
</table>

<pre><code class="language-jsonc">{
    &quot;labels&quot;: {
        &quot;cpu&quot;: &quot;procesador&quot;,
        &quot;gpu&quot;: &quot;&quot;,
        &quot;memory&quot;: &quot;ram&quot;
    }
}</code></pre>

<h2><code>formats</code> — Value Templates</h2>

<p>
  The <code>formats</code> map replaces a module's value with a template string. Placeholders <code>{field}</code> are substituted with the module's fields. Rules:
</p>

<ul>
  <li>Unknown placeholders render empty.</li>
  <li><code>{{</code> and <code>}}</code> escape literal braces.</li>
  <li>Modules without an entry use the default template <code>{value}</code> (the current output).</li>
</ul>

<table>
  <thead>
    <tr><th>Module</th><th>Fields</th><th>Example</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>every module</td>
      <td><code>{value}</code> (current output), <code>{key}</code> (module name)</td>
      <td><code>&quot;os&quot;: &quot;Sistema: {value}&quot;</code></td>
    </tr>
    <tr>
      <td><code>cpu</code></td>
      <td>
        <code>{brand}</code> raw brand<br>
        <code>{model}</code> cleaned brand (<code>(R)</code>/<code>(TM)</code>, <code>CPU @ ...</code>, core-count suffixes removed)<br>
        <code>{cores}</code> logical core count<br>
        <code>{freq}</code> frequency (<code>3.00 GHz</code>)
      </td>
      <td><code>&quot;cpu&quot;: &quot;{model} · {cores} núcleos · {freq}&quot;</code></td>
    </tr>
    <tr>
      <td><code>gpu</code></td>
      <td>
        <code>{name}</code> device name<br>
        <code>{vendor}</code> vendor (<code>NVIDIA</code>, <code>AMD</code>, <code>Intel</code>, <code>Apple</code>, ...)<br>
        <code>{model}</code> best-effort model (vendor, product-line prefix and VRAM stripped)<br>
        <code>{vram}</code> trailing VRAM (<code>6GB</code>), when present
      </td>
      <td><code>&quot;gpu&quot;: &quot;{vendor} {model}&quot;</code></td>
    </tr>
    <tr>
      <td><code>memory</code>, <code>swap</code></td>
      <td><code>{used}</code>, <code>{total}</code> (with unit), <code>{percent}</code> (bare number)</td>
      <td><code>&quot;memory&quot;: &quot;{used} / {total} ({percent}%)&quot;</code></td>
    </tr>
    <tr>
      <td><code>disk</code></td>
      <td>memory fields plus <code>{fs}</code> (filesystem name, when shown)</td>
      <td><code>&quot;disk&quot;: &quot;{used} en {fs}&quot;</code></td>
    </tr>
    <tr>
      <td><code>os</code></td>
      <td><code>{distro}</code>, <code>{version}</code>, <code>{arch}</code>, <code>{wsl}</code> (only with the WSL decoration)</td>
      <td><code>&quot;os&quot;: &quot;{distro} {version} ({arch})&quot;</code></td>
    </tr>
    <tr>
      <td><code>packages</code></td>
      <td>one field per package manager (<code>{pacman}</code>, <code>{aur}</code>, <code>{dpkg}</code>, ...), plus <code>{count}</code>/<code>{manager}</code> (first entry) and <code>{managers}</code> (joined names)</td>
      <td><code>&quot;packages&quot;: &quot;pkg: {pacman} · aur: {aur}&quot;</code></td>
    </tr>
    <tr>
      <td><code>battery</code></td>
      <td><code>{percent}</code> (bare number), <code>{state}</code> (<code>Charging</code>, <code>Discharging</code>, ...)</td>
      <td><code>&quot;battery&quot;: &quot;{percent}% [{state}]&quot;</code></td>
    </tr>
    <tr>
      <td><code>uptime</code></td>
      <td><code>{days}</code>, <code>{hours}</code>, <code>{mins}</code> (days derived from the hour count)</td>
      <td><code>&quot;uptime&quot;: &quot;{days}d {hours}h {mins}m&quot;</code></td>
    </tr>
    <tr>
      <td><code>datetime</code></td>
      <td><code>{date}</code>, <code>{time}</code></td>
      <td><code>&quot;datetime&quot;: &quot;{date} | {time}&quot;</code></td>
    </tr>
  </tbody>
</table>

<pre><code class="language-jsonc">{
    &quot;formats&quot;: {
        &quot;cpu&quot;: &quot;{model} ({cores}) @ {freq}&quot;,
        &quot;gpu&quot;: &quot;{vendor} {model}&quot;,
        &quot;memory&quot;: &quot;{value}&quot;
    }
}</code></pre>

<h2>Examples</h2>

<p>
  Source values and what the templates turn them into:
</p>

<table>
  <thead>
    <tr><th>Module</th><th>Raw value</th><th>Template</th><th>Result</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>cpu</code></td>
      <td><code>Intel(R) Core(TM) i5-7400 CPU @ 3.00GHz (4) @ 3.00 GHz</code></td>
      <td><code>{model}</code></td>
      <td><code>Intel Core i5-7400</code></td>
    </tr>
    <tr>
      <td><code>cpu</code></td>
      <td><code>AMD Ryzen 7 5800X 8-Core Processor (8) @ 4.70 GHz</code></td>
      <td><code>{model} ({cores}) @ {freq}</code></td>
      <td><code>AMD Ryzen 7 5800X (8) @ 4.70 GHz</code></td>
    </tr>
    <tr>
      <td><code>gpu</code></td>
      <td><code>NVIDIA GeForce GTX 1060 6GB</code></td>
      <td><code>{vendor} {model}</code></td>
      <td><code>NVIDIA GTX 1060</code></td>
    </tr>
    <tr>
      <td><code>gpu</code></td>
      <td><code>GP106 [GeForce GTX 1060 6GB]</code> (lspci)</td>
      <td><code>{model}</code></td>
      <td><code>GTX 1060</code></td>
    </tr>
  </tbody>
</table>

<h2>GPU Fields Per Platform</h2>

<p>
  GPU field extraction lives where the probe output lives — each platform knows the shape of its own output:
</p>

<table>
  <thead>
    <tr><th>Platform</th><th>Probe</th><th>Raw example</th><th>Extraction</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>Linux</td>
      <td><code>lspci -mm</code></td>
      <td><code>GP106 [GeForce GTX 1060 6GB]</code></td>
      <td><code>{name}</code> from the bracketed description, then the shared vendor/VRAM/model rules.</td>
    </tr>
    <tr>
      <td>Windows</td>
      <td><code>wmic</code> / PowerShell CIM</td>
      <td><code>NVIDIA GeForce GTX 1060 6GB</code></td>
      <td>Plain device name; vendor word and VRAM split off.</td>
    </tr>
    <tr>
      <td>macOS</td>
      <td><code>system_profiler</code> (Chipset Model)</td>
      <td><code>Apple M2 Pro</code></td>
      <td>Plain device name; vendor word and VRAM split off.</td>
    </tr>
  </tbody>
</table>

<p>
  The shared rules (vendor detection, trailing VRAM, model cleaning) live in <code>platform/shared/gpu.rs</code> and are covered by unit tests.
</p>

<h2>Escaping and Empty Fields</h2>

<pre><code class="language-jsonc">{
    &quot;formats&quot;: {
        // {{ and }} produce literal braces:
        &quot;os&quot;: &quot;{{ {value} }}&quot;,          // "{ Ubuntu 24.04 }"
        // unknown fields render empty:
        &quot;hostname&quot;: &quot;[{unknown}] {value}&quot; // "[ ] myhost"
    }
}</code></pre>

<h2>Related</h2>

<ul>
  <li><a href="CONFIGURATION.md">CONFIGURATION.md</a> — the rest of the config surface (modules, icons, colors, layout).</li>
  <li><a href="LAYOUTS.md">LAYOUTS.md</a> — the layouts these labels/formats apply to.</li>
</ul>
