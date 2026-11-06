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

<pre><code class="language-bash">xfetch extension install /path/to/xfetch-extension-name
xfetch extension install my-extension --repo https://github.com/user/extensions.git</code></pre>

<p>
  Installs and updates write an <code>.install.json</code> receipt next to the
  extension and keep the previous artifact as <code>.bak</code> for rollback.
</p>

<p>List, inspect and remove installed extensions:</p>

<pre><code class="language-bash">xfetch extension list
xfetch extension list --verbose
xfetch extension info &lt;name&gt;
xfetch extension search &lt;query&gt;
xfetch extension available
xfetch extension update &lt;name&gt;
xfetch extension update --all
xfetch extension update --all --check
xfetch extension rollback &lt;name&gt;
xfetch extension remove &lt;name&gt;</code></pre>

<p>
  <code>search</code> reads the extensions catalog (<code>index.json</code>),
  filters by name, description or runtime, and prints the matching install
  command. <code>available</code> lists the whole catalog, <code>--json</code>
  emits JSON, and <code>XFETCH_EXTENSION_INDEX</code> overrides the catalog URL.
</p>

<p>
  After a successful install, xfetch prints the
  <code>"config_providers"</code> entry, the declared capabilities and the
  manifest limits. <code>list --verbose</code> adds the manifest kind, runtime,
  version and whether the extension is enabled; <code>info</code> prints the
  details for one extension. Direct wasm files and URLs accept
  <code>--name</code> and <code>--manifest</code>; directory and repository
  installs use their own name and manifest.
</p>

<p>Installed binaries live in <code>~/.config/xfetch/extensions/</code> and are named
<code>xfetch-extension-&lt;name&gt;</code> (<code>.exe</code> on Windows).</p>

<h2>WebAssembly Extensions</h2>

<p>
  Extensions can be WebAssembly artifacts: the core detects the header, runs
  the guest in the sandbox and feeds it the resolved config JSON. Capabilities
  (for example reading an allowlisted config directory) come from the manifest,
  and limits are enforced by the runtime.
</p>

<pre><code class="language-bash">xfetch extension install ./extensions/wasm-night-mode
xfetch extension install ./extensions/wasm-updates-footer</code></pre>

<p>
  See <a href="WASM.md">WASM.md</a> for the manifest schema, host calls and
  examples.
</p>

<p>
  Pin an extension with <code>--sha256 &lt;64-hex-digest&gt;</code> from a
  channel independent of the artifact; the digest is enforced on downloaded,
  checked-in, built and direct-file wasm installs. A manifest
  <code>sha256</code> field is checked too, but it is integrity-only because
  it shares the artifact's origin. A mismatch aborts the install; an unpinned
  download prints an unverified integrity warning.
</p>

<p>
  Native extensions run as ordinary processes, <strong>unsandboxed</strong>,
  with your user privileges; only wasm extensions use the wasmtime sandbox.
</p>

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

<p>
  Extensions declare their own runtime budget via
  <code>xfetch_extension_api::with_timeout</code> (required by the official
  standard). As a safety net, the core can kill an extension after an
  optional per-extension deadline:
</p>

<pre><code class="language-jsonc">{
    "config_providers": [
        { "extension": "config-roulette", "timeout_secs": 5 }
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
