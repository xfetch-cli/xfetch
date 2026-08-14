<h1>Extensions</h1>

<p>
  Extensions hook into the xfetch lifecycle at <strong>config load time</strong>: they
  receive the fully resolved configuration via stdin and return a modified version via
  stdout. They can change layouts, modules, colors, icons, logos, or load an entirely
  different config file.
</p>

<p>
  Unlike plugins (which provide info lines or animate logos), extensions operate on the
  config itself before rendering happens.
</p>

<h2>Installation</h2>

<pre><code class="language-bash">xfetch extension install &lt;name&gt;</code></pre>

<p>Or install from a local path / repository:</p>

<pre><code class="language-bash">xfetch extension install /path/to/xfetch-extension-name</code></pre>

<p>List and remove installed extensions:</p>

<pre><code class="language-bash">xfetch extension list
xfetch extension remove &lt;name&gt;</code></pre>

<p>Installed binaries live in <code>~/.config/xfetch/extensions/</code> and are named
<code>xfetch-extension-&lt;name&gt;</code> (<code>.exe</code> on Windows).</p>

<h2>Activation</h2>

<p>
  Extensions are enabled with the <code>config_providers</code> field in the config.
  They run in declaration order, after the theme merge:
</p>

<pre><code class="language-jsonc">{
    "config_providers": [
        {
            "extension": "config-roulette",
            "args": {
                "routes": "~/.config/xfetch/routes.json",
                "strategy": "random"
            }
        }
    ]
}</code></pre>

<h2>Available Extensions</h2>

<table>
  <thead>
    <tr><th>Extension</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>layout-override</code></td>
      <td>Overrides the layout and/or modules at load time.</td>
    </tr>
    <tr>
      <td><code>config-roulette</code></td>
      <td>Picks a random (<code>"random"</code>) or per-day (<code>"daily"</code>) config from a JSON list of routes — every invocation can show a different look.</td>
    </tr>
  </tbody>
</table>

<h2>Protocol</h2>

<p>The core and extensions communicate with JSON over stdin/stdout:</p>

<p><strong>Request</strong></p>

<pre><code class="language-json">{
    "version": 1,
    "kind": "config_provider",
    "config": { ... },
    "args": { ... }
}</code></pre>

<p><strong>Response</strong></p>

<pre><code class="language-json">{
    "config": { ... }
}</code></pre>

<p>
  The extension receives the fully resolved xfetch configuration (defaults + config
  file + theme), modifies the fields it cares about, and returns the entire config
  object. Unchanged fields are preserved.
</p>

<p>
  Errors should be printed to stderr and the process should exit with a non-zero status.
</p>

<h2>Authoring</h2>

<p>
  Official extensions and the authoring guide are maintained in the
  <a href="https://github.com/xfetch-cli/extensions">xfetch-cli/extensions</a>
  repository. The shared wire protocol types live in
  <a href="https://github.com/xfetch-cli/api">xfetch-cli/api</a>.
</p>
