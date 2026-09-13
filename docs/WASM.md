<h1>WebAssembly Guests</h1>

<p>
  Plugins, effects and extensions can be WebAssembly artifacts instead of native
  executables. The core detects them by their binary header, runs them inside a
  sandboxed wasmtime runtime and keeps the same JSON protocol version 1, so the
  rest of the system (config, timeouts, list/remove commands) is unchanged.
</p>

<p>
  This page is the authoritative reference for the wasm runtime. Authoring
  guides and SDK notes live in
  <a href="https://github.com/xfetch-cli/api/tree/main/docs">xfetch-cli/api</a>.
</p>

<h2>Guest Shapes</h2>

<table>
  <thead>
    <tr><th>Shape</th><th>Target</th><th>Contract</th><th>Languages</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>Core module</td>
      <td><code>wasm32-wasip1</code></td>
      <td>JSON request on stdin, JSON response on stdout, host calls through the <code>xfetch.host_call</code> import</td>
      <td>Rust, C, Zig, Go, AssemblyScript, ...</td>
    </tr>
    <tr>
      <td>Component</td>
      <td>Component model (<code>wasm32</code>)</td>
      <td>Export <code>run</code> from the WIT world, typed host imports</td>
      <td>Python (<code>componentize-py</code>), JavaScript (<code>componentize-js</code>), Rust (<code>wit-bindgen</code>), ...</td>
    </tr>
  </tbody>
</table>

<p>
  Detection is content-based: an 8-byte header check distinguishes core modules
  (version 1) from components (version 13, layer 1). The file extension does
  not matter.
</p>

<h2>Quick Start</h2>

<pre><code class="language-bash"># Inspect an artifact without running it
xfetch wasm inspect ./xfetch-plugin-wasm-crypto.wasm

# Run it with a raw JSON request (useful for authors and scripts)
xfetch wasm run ./plugin.wasm --request '{"version":1,"kind":"info_provider"}'

# Print the WIT contract used by component guests
xfetch wasm wit

# Install from a local checkout, a remote repository or a prebuilt URL
xfetch plugin install ./plugins/wasm-pacman
xfetch plugin install wasm-crypto
xfetch plugin install https://example.com/releases/plugin.wasm
xfetch effects install ./effects/wasm-matrix
xfetch extension install ./extensions/wasm-night-mode</code></pre>

<h2>Manifest</h2>

<p>
  Capabilities and limits come from a JSON manifest resolved in this order:
  a sidecar next to the artifact (<code>xfetch-plugin-name.wasm</code> &rarr;
  <code>xfetch-plugin-name.json</code>), a custom section embedded in the
  binary under the name <code>xfetch:manifest</code>, or conservative
  defaults. No manifest still works for pure stdin/stdout guests, but grants
  nothing.
</p>

<p>
  Source repositories use the same schema in a file named
  <code>xfetch-plugin.json</code>, <code>xfetch-effect.json</code> or
  <code>xfetch-extension.json</code>; the installer copies it next to the
  artifact.
</p>

<pre><code class="language-json">{
  "manifest_version": 1,
  "name": "wasm-crypto",
  "version": "0.2.0",
  "description": "Example info provider",
  "kind": "info_provider",
  "runtime": "core",
  "build": "cargo build --release --target wasm32-wasip1",
  "artifact": "../../target/wasm32-wasip1/release/xfetch-plugin-wasm-crypto.wasm",
  "artifact_url": "https://github.com/user/repo/releases/latest/download/plugin.wasm",
  "capabilities": {
    "http": { "allow": ["https://api.example.com/*"] },
    "exec": { "allow": ["git"], "env": ["PATH"] },
    "fs": [
      "~/.config/xfetch",
      { "host": "~/.cache/xfetch", "guest": "/data", "mode": "rw" }
    ],
    "env": ["HOME", "USER"],
    "args": false
  },
  "limits": {
    "timeout_ms": 30000,
    "memory_mb": 256,
    "output_kb": 4096,
    "host_call_kb": 4096
  }
}</code></pre>

<h3>Manifest Fields</h3>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>manifest_version</code></td><td>number</td><td>Schema version; currently <code>1</code>.</td></tr>
    <tr><td><code>name</code></td><td>string</td><td>Guest name used in diagnostics and logs.</td></tr>
    <tr><td><code>version</code></td><td>string</td><td>Informational version string.</td></tr>
    <tr><td><code>description</code></td><td>string</td><td>Informational description.</td></tr>
    <tr><td><code>kind</code></td><td>string</td><td><code>info_provider</code>, <code>logo_animation</code>, <code>effect</code> or <code>config_provider</code>.</td></tr>
    <tr><td><code>runtime</code></td><td>string</td><td><code>core</code> or <code>component</code>; informational (detection reads the header).</td></tr>
    <tr><td><code>entry</code></td><td>string</td><td>Component export name; defaults to <code>run</code>.</td></tr>
    <tr><td><code>artifact</code></td><td>string</td><td>Relative path to a prebuilt artifact in the source directory.</td></tr>
    <tr><td><code>artifact_url</code></td><td>string</td><td>Absolute URL of a prebuilt artifact for remote installs that cannot build locally.</td></tr>
    <tr><td><code>build</code></td><td>string</td><td>Command executed in the source directory when no artifact is present.</td></tr>
  </tbody>
</table>

<h3>Capabilities</h3>

<p>
  Capabilities are deny-by-default. Empty or missing fields deny the
  corresponding operation.
</p>

<table>
  <thead>
    <tr><th>Capability</th><th>Field</th><th>Semantics</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>HTTP</td>
      <td><code>http.allow</code></td>
      <td>Glob patterns matched against the full URL (<code>*</code> and <code>?</code>). Redirects are followed manually and every hop is re-checked.</td>
    </tr>
    <tr>
      <td>Processes</td>
      <td><code>exec.allow</code></td>
      <td>Program file-name patterns. argv is passed verbatim; no shell is involved. <code>exec.env</code> allowlists which variables the guest may forward.</td>
    </tr>
    <tr>
      <td>Filesystem</td>
      <td><code>fs</code></td>
      <td>WASI preopens. Strings mount read-only at the expanded host path; objects choose <code>host</code>, <code>guest</code> and <code>mode</code> (<code>ro</code>/<code>rw</code>).</td>
    </tr>
    <tr>
      <td>Environment</td>
      <td><code>env</code></td>
      <td>Variable names exposed to the guest. <code>["*"]</code> forwards everything (discouraged).</td>
    </tr>
    <tr>
      <td>Arguments</td>
      <td><code>args</code></td>
      <td>When true, the guest receives its name as <code>argv[0]</code>; when false (default), <code>argv</code> is empty.</td>
    </tr>
  </tbody>
</table>

<p>
  <code>log</code> and <code>version</code> host operations are always
  available and need no manifest entry.
</p>

<h3>Limits</h3>

<table>
  <thead>
    <tr><th>Field</th><th>Default</th><th>Meaning</th></tr>
  </thead>
  <tbody>
    <tr><td><code>timeout_ms</code></td><td><code>30000</code></td><td>Wall-clock deadline for the whole invocation. The config <code>timeout_secs</code> value overrides it when set.</td></tr>
    <tr><td><code>memory_mb</code></td><td><code>256</code></td><td>Linear memory cap per memory.</td></tr>
    <tr><td><code>output_kb</code></td><td><code>4096</code></td><td>Cap for the JSON response written to stdout.</td></tr>
    <tr><td><code>host_call_kb</code></td><td><code>4096</code></td><td>Cap for a single host-call response (HTTP body, exec stdout/stderr).</td></tr>
  </tbody>
</table>

<p>
  Timeouts use wasmtime epochs: a background ticker advances the engine epoch
  every 10 ms and the guest traps when its deadline is reached. A guest that
  runs out of time is reported exactly like a native plugin timeout.
</p>

<h2>Guest Logs</h2>

<p>
  Guests can write diagnostics through the <code>log</code> host operation
  (<code>host.log</code> in components, <code>xfetch-guest-api</code> in Rust
  core modules). The host prints them to stderr with the guest name and level:
</p>

<pre><code>[wasm-crypto] [info] fetching prices</code></pre>

<p>
  By default only <code>warn</code> and <code>error</code> lines are shown, so
  informational guest chatter stays out of a normal fetch. Set
  <code>XFETCH_WASM_LOG_LEVEL</code> to <code>off</code>,
  <code>error</code>, <code>warn</code>, <code>info</code> or
  <code>debug</code> to change the threshold (for example
  <code>XFETCH_WASM_LOG_LEVEL=debug xfetch</code> when developing a guest).
</p>

<p>
  Raw writes to the guest's stderr (including panics) are always forwarded;
  only the structured host log is filtered.
</p>

<h2>Host Calls (Core Modules)</h2>

<p>
  Core modules import a single function from the <code>xfetch</code> module:
</p>

<pre><code>(func "host_call" (param i32 i32 i32 i32) (result i64))</code></pre>

<p>
  The guest writes the operation name and a JSON argument object into its
  memory and exports <code>xfetch_alloc(size) -&gt; ptr</code> so the host can
  place the response. The return value packs the response pointer and length:
  low 32 bits = pointer, high 32 bits = length. A return value of <code>0</code>
  means the host could not allocate the response.
</p>

<p>
  Every response is a JSON object:
</p>

<pre><code class="language-json">{ "ok": true,  "value": { } }
{ "ok": false, "error": { "kind": "denied", "message": "..." } }</code></pre>

<p>
  Error kinds are <code>denied</code>, <code>failed</code>,
  <code>timeout</code>, <code>too_large</code> and <code>unsupported</code>.
</p>

<table>
  <thead>
    <tr><th>Operation</th><th>Arguments</th><th>Result</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>http</code></td>
      <td><code>{ method, url, headers, body_base64?, timeout_ms? }</code></td>
      <td><code>{ status, headers: [[name, value], ...], body_base64 }</code></td>
    </tr>
    <tr>
      <td><code>exec</code></td>
      <td><code>{ program, args, stdin_base64?, env?, timeout_ms? }</code></td>
      <td><code>{ code, stdout_base64, stderr_base64 }</code></td>
    </tr>
    <tr>
      <td><code>log</code></td>
      <td><code>{ level, message }</code></td>
      <td><code>{ }</code></td>
    </tr>
    <tr>
      <td><code>version</code></td>
      <td><code>{ }</code></td>
      <td><code>{ runtime, protocol, xfetch, guest_kind }</code></td>
    </tr>
  </tbody>
</table>

<p>
  Rust authors should use the
  <a href="https://github.com/xfetch-cli/api/tree/main/crates/guest-api">xfetch-guest-api</a>
  crate, which implements the ABI, the allocator exports and typed helpers
  (<code>http_request</code>, <code>exec</code>, <code>log</code>,
  <code>protocol_version</code>). The core does not depend on this crate: the
  host bridge lives in the runtime, and only the guest adds it to its own
  <code>Cargo.toml</code>. Declaring it as <code>"0.2"</code> lets Cargo resolve
  the newest compatible 0.2.x release.
</p>

<h2>Components</h2>

<p>
  Components use typed imports instead of the JSON bridge. The contract lives
  in <code>wit/xfetch-runtime.wit</code> (also printable with
  <code>xfetch wasm wit</code>) and defines three worlds &mdash;
  <code>plugin</code>, <code>effect</code> and <code>extension</code> &mdash;
  that all export:
</p>

<pre><code class="language-wit">run: func(request: string) -> result&lt;string, string&gt;</code></pre>

<p>
  The imported <code>xfetch:runtime/host</code> interface exposes
  <code>fetch</code>, <code>exec</code>, <code>log</code> and
  <code>protocol-version</code>. WASI preview 2 is linked as well, so
  componentized Python and JavaScript runtimes get clocks, randomness and
  preopened filesystems.
</p>

<h2>Installation</h2>

<p>
  The installers detect wasm sources automatically:
</p>

<ul>
  <li>A source manifest (<code>xfetch-plugin.json</code>, <code>plugin.json</code>, ...) with a wasm runtime, <code>artifact</code>, <code>artifact_url</code> or <code>build</code>.</li>
  <li>A prebuilt artifact in <code>artifact</code>, <code>dist/&lt;name&gt;.wasm</code>, <code>&lt;name&gt;.wasm</code>, <code>xfetch-&lt;label&gt;-&lt;name&gt;.wasm</code> or a cargo wasm target directory.</li>
</ul>

<p>
  When only a <code>build</code> command exists it runs first, then the
  artifact is located and installed as <code>xfetch-&lt;label&gt;-&lt;name&gt;.wasm</code>
  plus its sidecar manifest. Native crates (<code>Cargo.toml</code> without
  wasm hints) keep the existing cargo build flow.
</p>

<p>
  Three install paths require no toolchain at all: a direct URL
  (<code>xfetch plugin install https://.../plugin.wasm</code>), a single local
  file (<code>xfetch plugin install ./plugin.wasm</code>) and a repository
  manifest with <code>artifact_url</code> (prebuilt GitHub releases).
</p>

<h2>Security Model</h2>

<p>
  Wasm guests start with no ambient authority: no filesystem, network,
  environment or process access. Every host operation is checked against the
  manifest, and limits bound time, memory and output. Native plugins are
  unchanged and keep their full process privileges; choosing wasm is choosing
  the sandbox.
</p>

<p>
  Redirects are re-validated per hop, <code>exec</code> never invokes a shell
  and clears the environment except for allowlisted names, and filesystem
  access is limited to explicit preopens.
</p>

<h2>Compilation Cache</h2>

<p>
  Compiled native code is cached under the platform cache directory
  (<code>~/.cache/xfetch/wasmtime</code> on Linux), keyed by the wasmtime
  version and the module hash. The first run of an artifact pays compilation;
  later runs reuse the cache. Set <code>WASMTIME_CACHE_DISABLE=1</code> to
  bypass it when debugging.
</p>

<h2>Building xfetch Without Wasm</h2>

<p>
  The runtime is behind the <code>wasm</code> feature, enabled by default:
</p>

<pre><code class="language-bash">cargo build --no-default-features   # smaller binary, no wasm runtime</code></pre>

<p>
  Such a binary still detects wasm artifacts and reports a clear error instead
  of trying to execute them as native processes.
</p>

<h2>Examples</h2>

<table>
  <thead>
    <tr><th>Guest</th><th>Language</th><th>Shape</th><th>Repository</th></tr>
  </thead>
  <tbody>
    <tr><td><code>wasm-crypto</code></td><td>Rust</td><td>Core module, allowlisted HTTP</td><td>xfetch-cli/plugins</td></tr>
    <tr><td><code>wasm-ip-geo</code></td><td>Python</td><td>Component, typed HTTP</td><td>xfetch-cli/plugins</td></tr>
    <tr><td><code>wasm-pacman</code></td><td>Go</td><td>Core module, allowlisted exec</td><td>xfetch-cli/plugins</td></tr>
    <tr><td><code>wasm-proc</code></td><td>C</td><td>Core module, read-only /proc</td><td>xfetch-cli/plugins</td></tr>
    <tr><td><code>wasm-matrix</code></td><td>Rust</td><td>Core module effect</td><td>xfetch-cli/effects</td></tr>
    <tr><td><code>wasm-python-pulse</code></td><td>Python</td><td>Component effect</td><td>xfetch-cli/effects</td></tr>
    <tr><td><code>wasm-night-mode</code></td><td>Rust</td><td>Core module extension</td><td>xfetch-cli/extensions</td></tr>
    <tr><td><code>wasm-updates-footer</code></td><td>Go</td><td>Core module extension, exec</td><td>xfetch-cli/extensions</td></tr>
    <tr><td><code>wasm-lang-labels</code></td><td>Python</td><td>Component extension, env</td><td>xfetch-cli/extensions</td></tr>
  </tbody>
</table>

<h2>Multi-Repository Development</h2>

<p>
  The ecosystem repositories consume the API crates from crates.io. For local
  development alongside this checkout they include a
  <code>[patch.crates-io]</code> section pointing at <code>../api</code>, and
  wasm-specific crates such as <code>xfetch-guest-api</code> are referenced by
  path. Remove those sections (or publish the crates) before building a
  standalone clone.
</p>

<h2>Troubleshooting</h2>

<table>
  <thead>
    <tr><th>Symptom</th><th>Likely cause</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>does not export _start</code></td>
      <td>A core module built as a library. Build it as a WASI command (<code>wasm32-wasip1</code> binary).</td>
    </tr>
    <tr>
      <td><code>host returned no buffer</code></td>
      <td>The guest does not export <code>xfetch_alloc</code>. Use <code>xfetch-guest-api</code> or implement the export.</td>
    </tr>
    <tr>
      <td><code>error: denied</code></td>
      <td>The manifest lacks the capability. Add an allowlist entry and reinstall.</td>
    </tr>
    <tr>
      <td><code>exceeded its timeout</code></td>
      <td>The guest ran past the deadline. Raise <code>timeout_ms</code> or <code>timeout_secs</code>, or optimize the guest.</td>
    </tr>
    <tr>
      <td><code>memory</code> limit traps</td>
      <td>Raise <code>memory_mb</code> in the manifest (Python components need headroom).</td>
    </tr>
    <tr>
      <td>Component fails with <code>Invalid plugin world</code></td>
      <td>The component does not export <code>run</code> from the expected world. Rebuild with the correct <code>-w</code> flag.</td>
    </tr>
  </tbody>
</table>
