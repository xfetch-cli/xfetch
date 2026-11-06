<h1>Command Line Interface</h1>

<p>
  This page is the complete reference for the <strong>xfetch</strong> command line:
  global options, output modes, runtime overrides, every subcommand, JSON schemas,
  exit codes and platform notes. For installation, see
  <a href="INSTALLATION.md">INSTALLATION.md</a>; for configuration keys, see
  <a href="CONFIGURATION.md">CONFIGURATION.md</a>.
</p>

<h2>Synopsis</h2>

<pre><code class="language-bash">xfetch [OPTIONS] [COMMAND]</code></pre>

<p>
  With no command, xfetch renders the fetch. Global options may appear before or
  after the subcommand. Unsupported flag combinations fail with exit code 2.
</p>

<h2>Global Options</h2>

<table>
  <thead>
    <tr><th>Option</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>-c, --config &lt;PATH&gt;</code></td><td>Use a custom config file.</td></tr>
    <tr><td><code>--gen-config</code></td><td>Generate a starter config. See <a href="GEN_CONFIG.md">GEN_CONFIG.md</a>.</td></tr>
    <tr><td><code>--logo &lt;NAME&gt;</code></td><td>Logo for the generated config. Only valid with <code>--gen-config</code>.</td></tr>
    <tr><td><code>--layout &lt;NAME&gt;</code></td><td>Override the layout at runtime. With <code>--gen-config</code>, select the generated layout instead.</td></tr>
    <tr><td><code>--theme &lt;NAME&gt;</code></td><td>Override the theme at runtime. The theme must exist.</td></tr>
    <tr><td><code>--list-layouts</code></td><td>Print every layout name accepted by <code>--layout</code> and exit.</td></tr>
    <tr><td><code>--list-modules</code></td><td>Print every module key accepted in <code>modules</code> and exit.</td></tr>
    <tr><td><code>--clean-cache</code></td><td>Remove the package and public-IP cache.</td></tr>
    <tr><td><code>--backup</code></td><td>Back up the config currently in use, including its theme and config-roulette routes.</td></tr>
    <tr><td><code>--restore</code></td><td>Restore the most recent config backup.</td></tr>
    <tr><td><code>--benchmark</code></td><td>Print benchmark lines for the info collection step.</td></tr>
    <tr><td><code>--daemon</code></td><td>Pin an animated fetch at the top of the terminal. See <a href="DAEMON.md">DAEMON.md</a>.</td></tr>
    <tr><td><code>--daemon-stop</code></td><td>Stop the animated-logo daemon.</td></tr>
    <tr><td><code>--no-daemon-live</code></td><td>Disable the live stats daemon for this run.</td></tr>
    <tr><td><code>--daemon-live-stop</code></td><td>Stop the live stats daemon.</td></tr>
    <tr><td><code>--daemon-live-reload</code></td><td>Force a hot reload of the live stats daemon.</td></tr>
    <tr><td><code>--json</code></td><td>Emit machine-readable JSON.</td></tr>
    <tr><td><code>--get &lt;KEY&gt;</code></td><td>Print only the requested module keys. Repeat the flag or use commas.</td></tr>
    <tr><td><code>--modules &lt;LIST&gt;</code></td><td>Override the rendered module list with a comma-separated key list.</td></tr>
    <tr><td><code>--only &lt;LIST&gt;</code></td><td>Alias for <code>--modules</code>.</td></tr>
    <tr><td><code>--color &lt;MODE&gt;</code></td><td><code>auto</code>, <code>always</code> or <code>never</code>. Default: <code>auto</code>.</td></tr>
    <tr><td><code>--no-color</code></td><td>Disable colors. Same as <code>--color never</code>.</td></tr>
    <tr><td><code>--offline</code></td><td>Disable network access: public IP, catalogs, update checks, logo fetch and remote addon installs.</td></tr>
    <tr><td><code>--no-cache</code></td><td>Bypass the package and public-IP cache for this run.</td></tr>
    <tr><td><code>--quiet</code></td><td>Suppress informational output. Command results are still printed.</td></tr>
    <tr><td><code>--verbose</code></td><td>Print extra diagnostics to stderr. Equivalent to <code>--log-level debug</code>.</td></tr>
    <tr><td><code>--log-level &lt;LEVEL&gt;</code></td><td><code>off</code>, <code>error</code>, <code>warn</code>, <code>info</code>, <code>debug</code> or <code>trace</code>. Overrides <code>--quiet</code>/<code>--verbose</code>.</td></tr>
    <tr><td><code>--log-json</code></td><td>Emit logs as JSON lines on stderr.</td></tr>
    <tr><td><code>--template &lt;FILE&gt;</code></td><td>Render the fetch with a template file.</td></tr>
    <tr><td><code>--format &lt;STRING&gt;</code></td><td>Render the fetch with an inline template.</td></tr>
    <tr><td><code>-h, --help</code></td><td>Print help.</td></tr>
    <tr><td><code>-V, --version</code></td><td>Print the version.</td></tr>
  </tbody>
</table>

<h2>Output Modes</h2>

<h3>Normal Render</h3>

<pre><code class="language-bash">xfetch
xfetch --layout right
xfetch --modules os,kernel,cpu,memory
xfetch --config ~/.config/xfetch/config.jsonc</code></pre>

<p>
  The renderer reads the config, applies the theme and config providers, collects
  only the required probes, and prints the selected layout.
</p>

<h3>JSON</h3>

<pre><code class="language-bash">xfetch --json
xfetch --json --benchmark
xfetch --json --modules os,kernel,cpu</code></pre>

<p>
  <code>--json</code> prints one object with <code>schema_version: 1</code>. It
  never prints ANSI escapes. <code>--benchmark</code> adds a <code>benchmark</code>
  array when present.
</p>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Notes</th></tr>
  </thead>
  <tbody>
    <tr><td><code>schema_version</code></td><td>number</td><td>Currently <code>1</code>.</td></tr>
    <tr><td><code>name</code></td><td>string</td><td><code>xfetch</code>.</td></tr>
    <tr><td><code>version</code></td><td>string</td><td>Binary version.</td></tr>
    <tr><td><code>os</code>, <code>kernel</code>, <code>hostname</code>, <code>user</code></td><td>string</td><td>System identity.</td></tr>
    <tr><td><code>shell</code>, <code>terminal</code>, <code>desktop</code></td><td>string</td><td>Session information.</td></tr>
    <tr><td><code>cpu</code></td><td>string</td><td>CPU model, cores and frequency.</td></tr>
    <tr><td><code>gpu</code></td><td>array of strings</td><td>One entry per detected GPU.</td></tr>
    <tr><td><code>memory</code>, <code>swap</code>, <code>battery</code>, <code>uptime</code></td><td>string</td><td>Formatted values.</td></tr>
    <tr><td><code>disks</code></td><td>array of strings</td><td>Formatted disk rows.</td></tr>
    <tr><td><code>packages</code></td><td>string</td><td>Combined package summary.</td></tr>
    <tr><td><code>packages_breakdown</code></td><td>array of objects</td><td><code>{"manager": "...", "count": "..."}</code>.</td></tr>
    <tr><td><code>datetime</code>, <code>local_ip</code>, <code>local_ip_v6</code>, <code>public_ip</code></td><td>string</td><td>Session/network values.</td></tr>
    <tr><td><code>network_interfaces</code></td><td>string</td><td>Interface summary.</td></tr>
    <tr><td><code>plugins</code></td><td>object</td><td>Map of <code>plugin:&lt;name&gt;</code> to output lines.</td></tr>
    <tr><td><code>benchmark</code></td><td>array of strings</td><td>Only with <code>--benchmark</code>.</td></tr>
  </tbody>
</table>

<h3>Specific Values</h3>

<pre><code class="language-bash">xfetch --get os
xfetch --get os,cpu,memory
xfetch --get version,name
xfetch --json --get os,cpu</code></pre>

<p>
  <code>--get</code> probes only the requested keys. A single key prints the raw
  value; multiple keys print <code>key=value</code> lines, with embedded newlines
  escaped as <code>\n</code>. With <code>--json</code> the output is an object:
</p>

<pre><code class="language-json">{
  "schema_version": 1,
  "values": {
    "cpu": "...",
    "os": "..."
  }
}</code></pre>

<p>
  Supported keys are the module keys from <code>--list-modules</code>, plus
  <code>version</code> and <code>name</code>. Patterns such as
  <code>packages:pacman</code> and <code>plugin:&lt;name&gt;</code> are accepted.
  Unknown keys fail with exit code 7.
</p>

<h3>Templates</h3>

<pre><code class="language-bash">xfetch --format 'os={{os}} kernel={{kernel}}'
xfetch --template ~/.config/xfetch/my.tmpl</code></pre>

<p>
  Templates replace <code>{{key}}</code> with the resolved value. Unknown keys
  render as an empty string. Supported placeholders include every module key,
  <code>packages:&lt;manager&gt;</code>, <code>plugin:&lt;name&gt;</code>,
  <code>palette</code>, <code>header</code>, <code>sep</code>,
  <code>name</code> and <code>version</code>. List values such as
  <code>gpu</code>, <code>disks</code>, <code>packages_breakdown</code> and
  <code>plugin:&lt;name&gt;</code> support loops and <code>join</code>.
</p>

<table>
  <thead>
    <tr><th>Syntax</th><th>Meaning</th></tr>
  </thead>
  <tbody>
    <tr><td><code>{{key}}</code></td><td>Value, or the comma-joined list.</td></tr>
    <tr><td><code>{{key|upper}}</code>, <code>lower</code>, <code>trim</code></td><td>Text transforms.</td></tr>
    <tr><td><code>{{key|count}}</code></td><td>List length or text length.</td></tr>
    <tr><td><code>{{key|join:" / "}}</code></td><td>Join a list with a separator.</td></tr>
    <tr><td><code>{{key|default:"fallback"}}</code></td><td>Fallback when empty.</td></tr>
    <tr><td><code>{{#if key}}...{{else}}...{{/if}}</code></td><td>Conditional block.</td></tr>
    <tr><td><code>{{#unless key}}...{{/unless}}</code></td><td>Inverted conditional.</td></tr>
    <tr><td><code>{{#each list}}...{{this}}...{{/each}}</code></td><td>Loop; <code>{{this}}</code> and <code>{{@index}}</code> are available.</td></tr>
  </tbody>
</table>

<pre><code class="language-text">{{name}} {{version}}
{{#if kernel}}kernel: {{kernel}}{{/if}}
{{#each gpu}}gpu[{{@index}}]: {{this}}{{/each}}
{{#each disks}}{{this}}\n{{/each}}
plugins: {{#each plugin:banner}}{{this}}{{/each}}
</code></pre>

<p>
  <code>--get</code> cannot be combined with <code>--template</code> or
  <code>--format</code>. <code>--json</code> cannot be combined with
  <code>--template</code> or <code>--format</code>. <code>--get</code>,
  <code>--template</code> and <code>--format</code> only apply to the default
  fetch and cannot be combined with a subcommand or daemon mode. The global
  <code>--json</code> flag is accepted by subcommands that support machine
  output.
</p>

<h3>Other JSON Outputs</h3>

<table>
  <thead>
    <tr><th>Command</th><th>Shape</th></tr>
  </thead>
  <tbody>
    <tr><td><code>--list-layouts --json</code></td><td>Array of layout names.</td></tr>
    <tr><td><code>modules --json</code>, <code>--list-modules --json</code></td><td>Array of <code>{key, description, platforms}</code>.</td></tr>
    <tr><td><code>theme list --json</code></td><td><code>{schema_version, themes: [{name, path, version, source, installed_unix}]}</code>.</td></tr>
    <tr><td><code>theme search|info --json</code></td><td>Registry metadata objects.</td></tr>
    <tr><td><code>theme install --json</code></td><td><code>{schema_version, status, name, version, path}</code>.</td></tr>
    <tr><td><code>theme update --json</code></td><td><code>{schema_version, check, themes: [{name, installed, available, update_available, status?}]}</code>.</td></tr>
    <tr><td><code>theme set|remove|export --json</code></td><td><code>{schema_version, status, theme, path?}</code>.</td></tr>
    <tr><td><code>logo list --json</code></td><td><code>{schema_version, logos: [{id, file, aliases}]}</code>.</td></tr>
    <tr><td><code>logo preview --json</code></td><td><code>{schema_version, id, file, aliases, art}</code>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; list --json</code></td><td><code>{schema_version, kind, addons: [...]}</code>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; info &lt;name&gt; --json</code></td><td>Flattened addon object with <code>schema_version</code>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; install --json</code></td><td><code>{schema_version, status, kind, name, path, source}</code> or an <code>installed</code> array with <code>--all</code>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; rollback|remove --json</code></td><td><code>{schema_version, status, kind, name}</code>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; update --check --json</code></td><td><code>{schema_version, kind, checks: [...]}</code>.</td></tr>
    <tr><td><code>run &lt;name&gt; --json</code></td><td><code>{schema_version, command, args, lines}</code>.</td></tr>
    <tr><td><code>update --check --json</code></td><td><code>{schema_version, status, current, latest, method?, command?, path?, update_available}</code>.</td></tr>
    <tr><td><code>update --rollback --json</code></td><td><code>{schema_version, status: "rolled-back", path}</code>.</td></tr>
    <tr><td><code>doctor --json</code></td><td><code>{schema_version, version, platform, checks, fixes}</code>.</td></tr>
    <tr><td><code>config print --json</code></td><td>Resolved config object.</td></tr>
    <tr><td><code>config get --json</code></td><td>The raw JSON value.</td></tr>
    <tr><td><code>config set|unset|reset|edit --json</code></td><td>Status object with <code>path</code>, <code>key</code>/<code>status</code> and optional <code>safety_backup</code>.</td></tr>
    <tr><td><code>config backups --json</code></td><td><code>{schema_version, backups: [{path, created_unix, created_utc, files, missing, size, compressed}]}</code>.</td></tr>
    <tr><td><code>config prune --json</code></td><td><code>{schema_version, kept, deleted, dry_run}</code>.</td></tr>
    <tr><td><code>--clean-cache --json</code></td><td><code>{"cleaned": true}</code>.</td></tr>
    <tr><td><code>--backup --json</code>, <code>config backup --json</code></td><td><code>{schema_version, path, files, missing, warnings, with_addons, compressed, encrypted}</code>.</td></tr>
    <tr><td><code>--restore --json</code>, <code>config restore --json</code></td><td><code>{schema_version, backup, dry_run, root, restored, missing, safety_backup, validation}</code>.</td></tr>
    <tr><td><code>--daemon-stop --json</code>, <code>--daemon-live-stop --json</code></td><td><code>{"stopped": bool}</code>.</td></tr>
    <tr><td><code>--gen-config --json</code></td><td><code>{"path": "..."}</code>.</td></tr>
    <tr><td><code>schema</code></td><td>JSON Schema object (or a map for <code>all</code>).</td></tr>
    <tr><td><code>self-uninstall --json</code></td><td><code>{schema_version, status, purged, removed}</code>.</td></tr>
  </tbody>
</table>

<h2>Runtime Overrides</h2>

<table>
  <thead>
    <tr><th>Flag</th><th>Effect</th></tr>
  </thead>
  <tbody>
    <tr><td><code>--layout &lt;NAME&gt;</code></td><td>Replaces <code>layout</code>. Validated against <code>--list-layouts</code>.</td></tr>
    <tr><td><code>--theme &lt;NAME&gt;</code></td><td>Selects the theme before the file/theme merge. The theme must exist.</td></tr>
    <tr><td><code>--modules &lt;LIST&gt;</code>, <code>--only &lt;LIST&gt;</code></td><td>Replaces the rendered module list. Comma-separated; groups are not supported by this override.</td></tr>
    <tr><td><code>--color</code>, <code>--no-color</code></td><td>Controls ANSI output. Images are not affected.</td></tr>
    <tr><td><code>--offline</code></td><td>Disables public IP, catalogs, updates, logo fetch and remote addon installs. Local addon installs still work.</td></tr>
    <tr><td><code>--no-cache</code></td><td>Bypasses the package/public-IP cache.</td></tr>
  </tbody>
</table>

<p>Precedence, from highest to lowest:</p>

<ol>
  <li>CLI overrides (<code>--layout</code>, <code>--modules</code>, <code>--no-color</code>, <code>--offline</code>, <code>--no-cache</code>)</li>
  <li>Config providers (extensions)</li>
  <li>Theme</li>
  <li>Config file</li>
  <li>Built-in defaults</li>
</ol>

<p>
  <code>--theme</code> is applied before the theme merge so the selected theme is
  the one that participates. <code>--color auto</code> strips ANSI when stdout is
  not a TTY; <code>always</code> keeps it, <code>never</code> and
  <code>--no-color</code> remove it.
</p>

<h2>Commands</h2>

<table>
  <thead>
    <tr><th>Command</th><th>Purpose</th></tr>
  </thead>
  <tbody>
    <tr><td><code>plugin</code> (alias <code>plugins</code>)</td><td>Manage plugins. See <a href="PLUGINS.md">PLUGINS.md</a>.</td></tr>
    <tr><td><code>effects</code> (alias <code>effect</code>)</td><td>Manage intro effects. See <a href="EFFECTS.md">EFFECTS.md</a>.</td></tr>
    <tr><td><code>extension</code> (alias <code>extensions</code>)</td><td>Manage config providers. See <a href="EXTENSIONS.md">EXTENSIONS.md</a>.</td></tr>
    <tr><td><code>addon &lt;kind&gt; &lt;action&gt;</code></td><td>Unified addon command. <code>kind</code> is <code>plugin</code>, <code>effect</code>, <code>extension</code> or <code>command</code>.</td></tr>
    <tr><td><code>command</code> (alias <code>commands</code>)</td><td>Manage CLI command addons: install, update, rollback, list, info, search and remove.</td></tr>
    <tr><td><code>run &lt;name&gt; [--] &lt;args&gt;</code></td><td>Run an installed command addon. <code>--render</code> uses the core layout and logo.</td></tr>
    <tr><td><code>theme</code></td><td>Search, install, update, list, set, remove or export themes. See <a href="THEMES.md">THEMES.md</a>.</td></tr>
    <tr><td><code>logo</code></td><td>List and preview logos from the ASCII logo catalog.</td></tr>
    <tr><td><code>wasm</code></td><td>Inspect and run WebAssembly guests. See <a href="WASM.md">WASM.md</a>.</td></tr>
    <tr><td><code>update</code></td><td>Check for and install a newer release.</td></tr>
    <tr><td><code>doctor</code></td><td>Diagnose the installation, configuration, terminal and addons. Supports <code>--fix</code>, <code>--bench</code> and <code>--image</code>.</td></tr>
    <tr><td><code>modules</code></td><td>List module keys (same registry as <code>--list-modules</code>).</td></tr>
    <tr><td><code>config</code></td><td>Validate, print, get, set, back up or restore configuration values.</td></tr>
    <tr><td><code>completions &lt;shell&gt;</code></td><td>Generate shell completions.</td></tr>
    <tr><td><code>man</code></td><td>Render the man page.</td></tr>
    <tr><td><code>schema</code></td><td>Print the JSON Schema for the CLI machine-readable outputs.</td></tr>
    <tr><td><code>paths</code></td><td>Print resolved config, addon, cache, backup and data paths.</td></tr>
    <tr><td><code>env</code></td><td>Print build, platform, environment, settings and path context.</td></tr>
    <tr><td><code>build-info</code></td><td>Print version, target, profile and enabled features.</td></tr>
    <tr><td><code>self-uninstall</code></td><td>Remove the binary and optionally its config, cache and data.</td></tr>
  </tbody>
</table>

<h2>Addon Commands</h2>

<p>
  The <code>plugin</code>, <code>effects</code>, <code>extension</code> and
  <code>command</code> families accept the same management actions. The unified
  form is equivalent:
</p>

<pre><code class="language-bash">xfetch addon plugin install banner
xfetch addon effect install decrypt
xfetch addon extension install config-roulette
xfetch addon command install github
xfetch command install github
xfetch plugin install banner
xfetch effects install decrypt
xfetch extension install config-roulette</code></pre>

<table>
  <thead>
    <tr><th>Action</th><th>Usage</th><th>Notes</th></tr>
  </thead>
  <tbody>
    <tr><td><code>install</code></td><td><code>install [PATH] [--repo &lt;URL&gt;] [--sha256 &lt;HEX&gt;] [--name &lt;NAME&gt;] [--manifest &lt;FILE&gt;] [--all] [--force]</code></td><td>Install from a catalog name, local directory, repository, direct wasm file or URL.</td></tr>
    <tr><td><code>update</code></td><td><code>update [NAME] [--all] [--check] [--force]</code></td><td>Rebuild native addons from the recorded source; keep <code>.bak</code> for rollback.</td></tr>
    <tr><td><code>rollback</code></td><td><code>rollback &lt;NAME&gt;</code></td><td>Restore the previous artifact and receipt.</td></tr>
    <tr><td><code>list</code></td><td><code>list [--verbose]</code></td><td>List installed addons. <code>--verbose</code> adds capabilities and limits.</td></tr>
    <tr><td><code>info</code></td><td><code>info &lt;NAME&gt;</code></td><td>Show metadata, capabilities, limits and the config snippet.</td></tr>
    <tr><td><code>search</code></td><td><code>search [QUERY]</code> (alias <code>available</code>)</td><td>Search the catalog; omit the query to list everything.</td></tr>
    <tr><td><code>remove</code></td><td><code>remove &lt;NAME&gt;</code></td><td>Remove the installed artifact and receipt.</td></tr>
  </tbody>
</table>

<p>
  <code>--json</code> is supported for every addon action: <code>install</code>
  returns the installed name and path (or a list with <code>--all</code>),
  <code>rollback</code> and <code>remove</code> return a status object,
  <code>list</code> and <code>info</code> return metadata, <code>search</code>
  returns the catalog and <code>update --check</code> returns version checks.
</p>

<h3>Addon JSON</h3>

<pre><code class="language-json">{
  "schema_version": 1,
  "kind": "plugin",
  "addons": [
    {
      "kind": "plugin",
      "name": "banner",
      "path": "/home/user/.config/xfetch/plugins/xfetch-plugin-banner",
      "runtime": "native",
      "manifest_kind": "plugin",
      "version": "0.2.0",
      "description": "...",
      "enabled": true,
      "capabilities": ["..."],
      "limits": "...",
      "config": ["..."],
      "manifest_source": null,
      "installed_source": "catalog"
    }
  ]
}</code></pre>

<p>
  <code>info --json</code> prints the same fields flattened with
  <code>schema_version</code>. <code>install --json</code> prints
  <code>{schema_version, status, kind, name, path, source}</code> (or an
  <code>installed</code> array with <code>--all</code>). <code>rollback --json</code>
  and <code>remove --json</code> print <code>{schema_version, status, kind, name}</code>.
  <code>update --check --json</code> prints:
</p>

<pre><code class="language-json">{
  "schema_version": 1,
  "kind": "plugin",
  "checks": [
    {
      "name": "banner",
      "installed": "0.2.0",
      "available": "0.2.1",
      "update_available": true,
      "compatible": true,
      "min_xfetch_version": null
    }
  ]
}</code></pre>

<h2>Command Addons</h2>

<p>
  Command addons expose extra CLI commands. They are installed and updated like
  the other addon families, but they run on demand:
</p>

<pre><code class="language-bash">xfetch command install github
xfetch command list
xfetch command info github
xfetch command update --all --check
xfetch command update github
xfetch command rollback github
xfetch command remove github

xfetch run github -- rust-lang/rust
xfetch run github -- https://github.com/rust-lang/rust
xfetch --json run github -- rust-lang/rust
xfetch --layout right run github --render -- rust-lang/rust</code></pre>

<p>
  The command receives positional arguments and returns plain text, one field
  per line. The core prints the text as-is by default; <code>--render</code>
  passes the lines through the configured layout and logo. <code>--json</code>
  wraps the output as
  <code>{schema_version, command, args, lines}</code>.
</p>

<p>
  Official command addons live in
  <code>https://github.com/xfetch-cli/commands</code>. The catalog is at
  <code>.../main/index.json</code> (<code>XFETCH_COMMAND_INDEX</code> overrides
  it); <code>XFETCH_COMMAND_REPO</code> overrides the install repository and
  <code>XFETCH_COMMAND_DEV_DIR</code> adds a development directory.
  <code>--offline</code> blocks catalog and remote installs.
</p>

<p>
  Wasm command addons use the <code>command</code> world in
  <code>xfetch:runtime@0.1.0</code>; the request is JSON and the response is
  plain text. Native commands may read argv and/or the JSON request on stdin.
  See the <a href="WASM.md">WASM</a> guide and the API
  <code>command-sdk.md</code>.
</p>

<h2>Theme and Logo Commands</h2>

<h3>Theme Registry</h3>

<pre><code class="language-bash">xfetch theme search dark
xfetch theme info nord
xfetch theme install nord
xfetch theme install nord --force
xfetch theme update nord
xfetch theme update --all --check
xfetch theme list
xfetch theme list --json
xfetch theme set dracula
xfetch theme remove dracula
xfetch theme export my-theme
xfetch --theme dracula</code></pre>

<p>
  <code>theme search</code>, <code>theme info</code>, <code>theme install</code>
  and <code>theme update</code> use the registry at
  <code>https://raw.githubusercontent.com/xfetch-cli/themes/main/index.json</code>.
  Override it with <code>XFETCH_THEME_INDEX</code>. Installed themes write a
  <code>.&lt;name&gt;.xfetch-meta.json</code> sidecar with the installed version
  and source; <code>theme list</code> includes that metadata.
  <code>theme update --check</code> exits 1 when an update is available.
  <code>--offline</code> blocks registry access.
</p>

<p>
  <code>theme list --json</code> prints a <code>themes</code> array with
  <code>{name, path, version, source, installed_unix}</code>.
  <code>theme search --json</code> and <code>theme info --json</code> return
  registry metadata. <code>theme install --json</code> returns
  <code>{schema_version, status, name, version, path}</code>.
  <code>theme update --json</code> returns a per-theme
  <code>{name, installed, available, update_available, status}</code> list.
</p>

<h3>Logo Catalog</h3>

<pre><code class="language-bash">xfetch logo list
xfetch logo list linux
xfetch logo list --json
xfetch logo preview arch
xfetch logo preview arch --json</code></pre>

<p>
  Logos come from
  <code>https://raw.githubusercontent.com/xfetch-cli/logos/main</code>
  (<code>XFETCH_LOGOS_URL</code> overrides the base URL). <code>logo list</code>
  filters by id, alias or file; <code>logo preview</code> prints the validated
  ASCII art. <code>--json</code> returns
  <code>{schema_version, logos:[{id,file,aliases}]}</code> for lists and
  <code>{schema_version, id, file, aliases, art}</code> for a preview.
  <code>--offline</code> blocks catalog access.
</p>

<h2>WebAssembly Commands</h2>

<pre><code class="language-bash">xfetch wasm inspect ./plugin.wasm
xfetch wasm inspect ./plugin.wasm --json
xfetch wasm run ./plugin.wasm --request '{"version":1,"kind":"info_provider"}'
xfetch wasm run ./plugin.wasm --request-file req.json --timeout 2 --kind effect
xfetch wasm wit</code></pre>

<table>
  <thead>
    <tr><th>Action</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>inspect &lt;FILE&gt;</code></td><td>Report kind, manifest, capabilities and limits without executing. Global <code>--json</code> switches to JSON.</td></tr>
    <tr><td><code>run &lt;FILE&gt;</code></td><td>Run a guest with a raw JSON request. Flags: <code>--request</code>, <code>--request-file</code>, <code>--timeout</code>, <code>--kind plugin|effect|extension</code>.</td></tr>
    <tr><td><code>wit</code></td><td>Print the canonical WIT package.</td></tr>
  </tbody>
</table>

<h2>Update Command</h2>

<pre><code class="language-bash">xfetch update --check
xfetch update --check --json
xfetch update
xfetch update --prebuilt
xfetch update --prebuilt --bin-dir ~/.local/bin
xfetch update --rollback
xfetch update --rollback --bin-dir ~/.local/bin</code></pre>

<table>
  <thead>
    <tr><th>Flag</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>--check</code></td><td>Only report; exit 1 when a newer release exists.</td></tr>
    <tr><td><code>--prebuilt</code></td><td>Install the prebuilt release in place (Unix and Windows).</td></tr>
    <tr><td><code>--bin-dir &lt;DIR&gt;</code></td><td>Directory holding the binary to replace (with <code>--prebuilt</code>).</td></tr>
    <tr><td><code>-y, --yes</code></td><td>Assume yes for prompts.</td></tr>
    <tr><td><code>--rollback</code></td><td>Restore the previous binary from <code>xfetch.bak</code>/<code>xfetch.exe.bak</code>.</td></tr>
  </tbody>
</table>

<p>
  <code>--json</code> requires <code>--check</code> for update checks; it also
  works with <code>--rollback</code>. Package-managed installs (Homebrew, cargo,
  winget, AUR) are delegated to their package manager instead of replacing the
  binary.
</p>

<pre><code class="language-json">{
  "schema_version": 1,
  "status": "update-available",
  "current": "1.0.2",
  "latest": "1.0.3",
  "method": "prebuilt",
  "command": "xfetch update --prebuilt",
  "update_available": true
}</code></pre>

<h2>Doctor Command</h2>

<pre><code class="language-bash">xfetch doctor
xfetch doctor --json
xfetch doctor --network
xfetch doctor --json --network
xfetch doctor --fix
xfetch doctor --fix --json
xfetch doctor --bench
xfetch doctor --bench --json
xfetch doctor --image</code></pre>

<p>
  <code>doctor</code> checks the binary, config file, theme, layout, terminal
  size, image protocol, logo width/gap geometry, addon directories, install
  receipts and cache. <code>--network</code> additionally checks the catalogs and
  the update API. It exits 1 when any check fails. <code>--quiet</code> hides
  successful checks. <code>--fix</code> applies safe fixes: it generates a
  missing config and removes an invalid cache file. <code>--bench</code> runs
  the info collection benchmark and includes it in the report.
  <code>--image</code> prints a preview of the configured logo/image when
  stdout is a terminal. Applied fixes and benchmark lines are included in JSON
  output.
</p>

<pre><code class="language-json">{
  "schema_version": 1,
  "version": "1.0.3",
  "platform": {
    "os": "linux",
    "arch": "x86_64",
    "family": "unix",
    "details": [{"name": "Distribution", "value": "Arch Linux"}]
  },
  "checks": [
    {
      "category": "config",
      "name": "file",
      "status": "ok",
      "detail": "/home/user/.config/xfetch/config.jsonc"
    }
  ],
  "fixes": []
}</code></pre>

<p><code>status</code> is <code>ok</code>, <code>warn</code> or <code>fail</code>.</p>

<h2>Modules Command</h2>

<pre><code class="language-bash">xfetch --list-modules
xfetch modules
xfetch modules --json</code></pre>

<p>
  The registry includes plain keys (<code>os</code>, <code>cpu</code>,
  <code>memory</code>, ...), patterns (<code>packages:&lt;manager&gt;</code>,
  <code>plugin:&lt;name&gt;</code>) and presentation keys
  (<code>palette</code>, <code>header</code>, <code>sep</code>).
</p>

<pre><code class="language-json">[
  {
    "key": "os",
    "description": "Operating system name and architecture",
    "platforms": "all"
  }
]</code></pre>

<h2>Config Commands</h2>

<pre><code class="language-bash">xfetch config validate
xfetch config print
xfetch config print --raw
xfetch config print --json
xfetch config get layout
xfetch config get colors.cpu
xfetch config set layout right
xfetch config set modules.1 kernel
xfetch config set colors.cpu red
xfetch config unset colors.cpu
xfetch config reset --yes
xfetch config edit
xfetch config backups
xfetch config prune --keep 5
xfetch --config ./config.jsonc config set theme dracula</code></pre>

<table>
  <thead>
    <tr><th>Action</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>validate</code></td><td>Parse JSONC strictly, validate the schema and check that the referenced theme exists.</td></tr>
    <tr><td><code>print</code></td><td>Print the resolved config after defaults, theme, providers and CLI overrides. <code>--raw</code> prints the stored file instead.</td></tr>
    <tr><td><code>get &lt;KEY&gt;</code></td><td>Read one dotted key path. Array indices such as <code>modules.0</code> are supported.</td></tr>
    <tr><td><code>set &lt;KEY&gt; &lt;VALUE&gt;</code></td><td>Set one value. Dotted paths and array indices are supported. Existing nested values are replaced in place, preserving surrounding comments; missing values fall back to a semantic rewrite. The result is validated before writing.</td></tr>
    <tr><td><code>unset &lt;KEY&gt;</code></td><td>Remove an object key or array element. Top-level keys preserve the rest of the file; nested removals may rewrite it.</td></tr>
    <tr><td><code>reset</code></td><td>Write the built-in defaults after a safety backup. Requires <code>--yes</code> in non-interactive runs.</td></tr>
    <tr><td><code>edit</code></td><td>Open the config in <code>$VISUAL</code> or <code>$EDITOR</code> after a safety backup, then validate it.</td></tr>
    <tr><td><code>backups</code></td><td>List saved backups with timestamps, file counts and sizes.</td></tr>
    <tr><td><code>prune</code></td><td>Delete old regular backups, keeping the newest N. Safety backups are preserved. <code>--dry-run</code> only lists them.</td></tr>
  </tbody>
</table>

<p>
  <code>config set --json</code>, <code>config unset --json</code>,
  <code>config reset --json</code> and <code>config edit --json</code> print
  status objects. <code>config get --json</code> prints the raw JSON value.
  <code>config backups --json</code> and <code>config prune --json</code> return
  structured backup metadata.
</p>

<h2>Backup and Restore</h2>

<p>
  <code>xfetch --backup</code> snapshots the config currently in use. The
  backup adapts to the installed setup:
</p>

<ul>
  <li>The main config file.</li>
  <li>Active themes for the main config and for every route config, recursively.</li>
  <li>For <code>config-roulette</code> (and any provider with a
  <code>routes</code> argument): the routes file and every referenced route
  config.</li>
  <li>Nested provider routes, up to a safe depth.</li>
  <li>Other provider arguments that look like local file paths and exist.</li>
  <li>Referenced logo/ASCII assets.</li>
  <li>With <code>--with-addons</code>: installed addon artifacts, sidecar
  manifests and install receipts.</li>
  <li>A resolved-config snapshot, missing files and parse warnings.</li>
</ul>

<pre><code class="language-bash"># Shortcuts
xfetch --backup
xfetch --restore

# Explicit commands
xfetch config backup
xfetch config backup --output ~/backups
xfetch config backup --output ~/backups/my-config.json
xfetch config backup --with-addons
xfetch config backup --compress
xfetch config backup --encrypt --passphrase-file ~/.config/xfetch/backup.pass
xfetch config restore
xfetch config restore --from ~/backups/my-config.json
xfetch config restore --from ~/backups --dry-run
xfetch config restore --root ~/restored
xfetch config restore --from backup.enc --passphrase-file backup.pass
xfetch config backups
xfetch config prune --keep 5
xfetch config prune --keep 5 --dry-run
xfetch --json --backup
xfetch --json --restore</code></pre>

<p>
  By default backups live in the xfetch config directory under
  <code>backups/</code> (usually <code>~/.config/xfetch/backups</code> on
  Linux). <code>--output</code> accepts a file or a directory. Without
  <code>--from</code>, <code>config restore</code> uses the newest backup by
  the bundle's own timestamp. <code>config backups</code> lists saved backups;
  <code>config prune --keep N</code> deletes older regular backups and keeps
  safety backups untouched.
</p>

<p>
  Restore always writes a safety backup named <code>pre-restore-*.json</code>
  before touching the files. Files are written atomically and parent
  directories are created. <code>--dry-run</code> prints the target paths
  without writing. Missing entries recorded at backup time are skipped. The
  restored main config is validated and the result is reported in the
  <code>validation</code> field.
</p>

<p>
  With <code>--config &lt;PATH&gt;</code>, the main config file is restored to
  that path while auxiliary files return to their original locations.
  <code>--root &lt;DIR&gt;</code> remaps every file under the original config
  directory into a new root, which is useful for restoring a complete setup
  elsewhere.
</p>

<p>
  <code>--compress</code> writes gzip-compressed backups. <code>--encrypt</code>
  writes a passphrase-encrypted backup using Argon2id and
  XChaCha20-Poly1305; the passphrase comes from
  <code>--passphrase-file</code> or <code>XFETCH_BACKUP_PASSPHRASE</code>.
  Restore detects plain JSON, gzip and encrypted containers automatically.
</p>

<p>
  Runtime overrides such as <code>--theme</code>, <code>--layout</code> and
  <code>--modules</code> are not persisted in the backup; the stored config and
  its referenced files are captured. For config-roulette, the
  <code>resolved_config</code> field is a snapshot of one invocation, while the
  raw routes are preserved for a faithful restore.
</p>

<p>
  Backups are JSON text (optionally compressed/encrypted) and may contain
  sensitive configuration values. On Unix the backup file is written with mode
  <code>0600</code>. Store encrypted backups and passphrase files securely.
</p>

<pre><code class="language-json">{
  "schema_version": 1,
  "created_unix": 1793987313,
  "created_unix_ms": 1793987313123,
  "created_utc": "2026-11-06T18:48:33Z",
  "xfetch_version": "1.0.3",
  "config_path": "/home/user/.config/xfetch/config.jsonc",
  "theme": "nord",
  "theme_path": "/home/user/.config/xfetch/themes/nord.jsonc",
  "config_providers": ["config-roulette"],
  "files": [
    {"role": "config", "path": "...", "content": "..."},
    {"role": "theme", "name": "nord", "path": "...", "content": "..."},
    {"role": "routes", "name": "config-roulette", "path": "...", "content": "..."},
    {"role": "route-config", "name": "a", "path": "...", "content": "..."},
    {"role": "logo", "path": "...", "encoding": "base64", "content": "..."},
    {"role": "addon-artifact", "name": "plugin:banner", "encoding": "base64", "content": "..."}
  ],
  "missing": [],
  "warnings": [],
  "resolved_config": {}
}</code></pre>

<h2>Schema Command</h2>

<pre><code class="language-bash">xfetch schema
xfetch schema fetch
xfetch schema backup
xfetch schema addon
xfetch schema update
xfetch schema doctor
xfetch schema config</code></pre>

<p>
  <code>schema</code> prints JSON Schema documents for the machine-readable
  outputs. <code>xfetch schema</code> prints all schemas in one object;
  <code>xfetch schema &lt;kind&gt;</code> prints one schema. The schemas are
  descriptive contracts for the <code>schema_version: 1</code> outputs.
</p>

<h2>Self-Uninstall</h2>

<pre><code class="language-bash">xfetch self-uninstall
xfetch self-uninstall --purge
xfetch self-uninstall --purge --yes</code></pre>

<p>
  <code>self-uninstall</code> removes the running binary and its update backup.
  <code>--purge</code> also removes the xfetch config directory (including
  themes, addons and backups), cache and data directories. Without
  <code>--yes</code>, it asks for confirmation when stdin is a terminal and
  refuses in non-interactive runs.
</p>

<h2>Shell Integration</h2>

<h3>Completions</h3>

<pre><code class="language-bash">xfetch completions bash
xfetch completions zsh
xfetch completions fish
xfetch completions elvish
xfetch completions powershell</code></pre>

<p>Examples:</p>

<pre><code class="language-bash"># Bash
xfetch completions bash > ~/.local/share/bash-completion/completions/xfetch

# Zsh
xfetch completions zsh > ~/.zfunc/_xfetch

# Fish
xfetch completions fish > ~/.config/fish/completions/xfetch.fish</code></pre>

<h3>Man Page</h3>

<pre><code class="language-bash">xfetch man
xfetch man --output ~/.local/share/man/man1/xfetch.1</code></pre>

<h2>Exit Codes</h2>

<table>
  <thead>
    <tr><th>Code</th><th>Meaning</th></tr>
  </thead>
  <tbody>
    <tr><td><code>0</code></td><td>Success.</td></tr>
    <tr><td><code>1</code></td><td>Generic failure, doctor failures, or <code>update --check</code> finding a newer release.</td></tr>
    <tr><td><code>2</code></td><td>Usage error: invalid flag, invalid combination or clap parse error.</td></tr>
    <tr><td><code>3</code></td><td>Configuration error.</td></tr>
    <tr><td><code>4</code></td><td>Network error or a request blocked by <code>--offline</code>.</td></tr>
    <tr><td><code>5</code></td><td>Addon error.</td></tr>
    <tr><td><code>6</code></td><td>Update error.</td></tr>
    <tr><td><code>7</code></td><td>Not found: unknown module, config key or addon.</td></tr>
  </tbody>
</table>

<h2>Environment Variables</h2>

<table>
  <thead>
    <tr><th>Variable</th><th>Effect</th></tr>
  </thead>
  <tbody>
    <tr><td><code>XFETCH_PLUGIN_REPO</code></td><td>Default plugin install repository.</td></tr>
    <tr><td><code>XFETCH_EFFECT_REPO</code></td><td>Default effect install repository.</td></tr>
    <tr><td><code>XFETCH_EXTENSION_REPO</code></td><td>Default extension install repository.</td></tr>
    <tr><td><code>XFETCH_PLUGIN_INDEX</code></td><td>Plugin catalog URL or local path.</td></tr>
    <tr><td><code>XFETCH_EFFECT_INDEX</code></td><td>Effect catalog URL or local path.</td></tr>
    <tr><td><code>XFETCH_EXTENSION_INDEX</code></td><td>Extension catalog URL or local path.</td></tr>
    <tr><td><code>XFETCH_LOGOS_URL</code></td><td>Base URL for the logo catalog.</td></tr>
    <tr><td><code>XFETCH_THEME_INDEX</code></td><td>Theme registry index URL or local path.</td></tr>
    <tr><td><code>XFETCH_COMMAND_INDEX</code></td><td>Command addon catalog URL or local path.</td></tr>
    <tr><td><code>XFETCH_COMMAND_REPO</code></td><td>Default command addon install repository.</td></tr>
    <tr><td><code>XFETCH_COMMAND_DEV_DIR</code></td><td>Extra development directory for command lookup.</td></tr>
    <tr><td><code>XFETCH_BACKUP_PASSPHRASE</code></td><td>Passphrase for encrypted config backups.</td></tr>
    <tr><td><code>VISUAL</code>, <code>EDITOR</code></td><td>Editor used by <code>xfetch config edit</code>.</td></tr>
    <tr><td><code>XFETCH_PLUGIN_DEV_DIR</code></td><td>Extra development directory for plugin lookup.</td></tr>
    <tr><td><code>XPM_AUR_STATE</code></td><td>Override the xpm-aur state file used for package counts.</td></tr>
    <tr><td><code>XDG_CONFIG_HOME</code></td><td>Config root (default <code>~/.config</code>).</td></tr>
    <tr><td><code>XDG_CACHE_HOME</code></td><td>Cache root (default <code>~/.cache</code>).</td></tr>
    <tr><td><code>XDG_STATE_HOME</code></td><td>State root (default <code>~/.local/state</code>).</td></tr>
    <tr><td><code>KITTY_WINDOW_ID</code></td><td>Enables the Kitty graphics protocol when set.</td></tr>
    <tr><td><code>TERM_PROGRAM</code>, <code>LC_TERMINAL</code></td><td>Select iTerm-compatible inline images when supported.</td></tr>
  </tbody>
</table>

<h2>Platform Notes</h2>

<table>
  <thead>
    <tr><th>Platform</th><th>Notes</th></tr>
  </thead>
  <tbody>
    <tr><td>Linux</td><td>Config under <code>~/.config/xfetch</code>. Package counts read local databases. Daemon mode uses a terminal scroll region.</td></tr>
    <tr><td>macOS</td><td>Config under <code>~/Library/Application Support/xfetch</code> with a symlink from the XDG path. Homebrew installs are updated through Homebrew.</td></tr>
    <tr><td>Windows</td><td>Config under <code>%APPDATA%\xfetch</code>. The updater uses <code>self-replace</code>; winget installs are delegated to <code>winget upgrade</code>. Daemon mode uses a worker process instead of <code>fork</code>.</td></tr>
  </tbody>
</table>

<p>
  Images use the Kitty graphics protocol when available, iTerm inline images in
  compatible terminals, and half blocks otherwise. Text colors are controlled by
  <code>--color</code>; images are not affected by <code>--no-color</code>.
</p>

<h2>Examples</h2>

<pre><code class="language-bash"># Normal fetch
xfetch

# Machine-readable output
xfetch --json
xfetch --json --benchmark

# Specific values
xfetch --get os,cpu,memory
xfetch --get version

# Runtime overrides
xfetch --layout right --theme dracula
xfetch --modules os,kernel,cpu,memory --no-color
xfetch --format '{{os}} | {{kernel}} | {{uptime}}'
xfetch --format '{{#if gpu}}gpu: {{gpu|join:", "}}{{else}}no gpu{{/if}}'

# Diagnostics
xfetch doctor
xfetch doctor --network --json
xfetch doctor --fix

# Configuration
xfetch config validate
xfetch config print
xfetch config get layout
xfetch config set layout right
xfetch config set modules.1 kernel
xfetch config unset colors.cpu
xfetch config reset --yes
xfetch config edit
xfetch config backups
xfetch config prune --keep 5
xfetch --backup
xfetch --restore
xfetch config backup --output ~/backups --compress
xfetch config backup --output ~/backups/secure.enc --encrypt --passphrase-file pass.txt
xfetch config restore --from ~/backups --dry-run
xfetch config restore --root ~/restored

# Themes and logos
xfetch theme search dark
xfetch theme install nord
xfetch theme update --all --check
xfetch logo list
xfetch logo preview arch

# Command addons
xfetch command install github
xfetch run github -- rust-lang/rust
xfetch --json run github -- rust-lang/rust
xfetch --layout right run github --render -- rust-lang/rust

# Addons
xfetch addon plugin install banner
xfetch plugin update --all --check
xfetch addon plugin list --json
xfetch effects install decrypt
xfetch extension install config-roulette

# Shell integration
xfetch completions zsh
xfetch man --output ~/.local/share/man/man1/xfetch.1
xfetch schema fetch

# Updates
xfetch update --check
xfetch update --check --json
xfetch update
xfetch update --rollback

# Remove xfetch
xfetch self-uninstall --purge --yes</code></pre>

<h2>Security Notes</h2>

<ul>
  <li>Native addons run unsandboxed with the current user's privileges.</li>
  <li>Wasm addons use the deny-by-default manifest policy with time, memory, output, host-call and capability limits.</li>
  <li><code>--sha256</code> on install is the provenance pin; it overrides the manifest digest.</li>
  <li>Catalog fetches are bounded and schema-checked.</li>
  <li>Config backups contain the raw config, referenced theme/route files and, with <code>--with-addons</code>, addon artifacts. Store them accordingly. On Unix they are written with mode <code>0600</code>.</li>
  <li>Encrypted backups use Argon2id and XChaCha20-Poly1305. Pass the passphrase through <code>--passphrase-file</code> or <code>XFETCH_BACKUP_PASSPHRASE</code>; do not put it directly on the command line.</li>
  <li><code>--offline</code> blocks xfetch network calls, but native addons are trusted code and can still make their own network requests. Wasm addons remain governed by the manifest policy.</li>
  <li>Never place secrets in config files, addon manifests or command lines; prefer environment variables supplied by your shell or secret manager.</li>
</ul>

<h2>Logging</h2>

<pre><code class="language-bash">xfetch --log-level debug
xfetch --log-level trace --verbose
xfetch --log-json --log-level debug
xfetch --log-level warn --quiet
xfetch --log-level off</code></pre>

<p>
  Log levels are <code>off</code>, <code>error</code>, <code>warn</code>,
  <code>info</code>, <code>debug</code> and <code>trace</code>. Fatal CLI errors
  are always printed. <code>--quiet</code> maps to <code>warn</code>;
  <code>--verbose</code> maps to <code>debug</code>; an explicit
  <code>--log-level</code> wins. <code>--log-json</code> writes
  <code>{"level": "...", "target": "xfetch", "message": "..."}</code> lines to
  stderr, keeping stdout clean for command results and <code>--json</code>.
</p>

<h2>Diagnostics Commands</h2>

<pre><code class="language-bash">xfetch paths
xfetch paths --json
xfetch env
xfetch env --json
xfetch build-info
xfetch build-info --json</code></pre>

<p>
  <code>paths</code> prints the resolved config file, config directory, themes,
  plugins, effects, extensions, backups, cache and data paths.
  <code>env</code> adds build information, platform details, environment
  variables (terminal, session, XDG), runtime settings and the same path map.
  <code>build-info</code> prints the version, target OS/architecture, build
  profile and enabled features (wasm, update, encryption). All three support
  <code>--json</code>.
</p>

<h2>Sandbox Test</h2>

<p>
  Build the release binary and run the end-to-end sandbox:
</p>

<pre><code class="language-bash">cargo build --release --locked
bash scripts/test-cli-sandbox.sh</code></pre>

<p>
  The script uses an isolated <code>HOME</code> and XDG directories, exercises
  the output modes, overrides, templates, config editing, doctor (including
  <code>--fix</code>), themes and logos, addons, wasm tooling, completions, man
  page, schema, backup/restore (compression, encryption, assets, addons and
  root remapping), update rollback, self-uninstall and a config-roulette setup
  with a fake extension. It prints the sandbox path and a pass/fail summary.
</p>

<h2>Related Documents</h2>

<ul>
  <li><a href="INSTALLATION.md">Installation</a></li>
  <li><a href="CONFIGURATION.md">Configuration</a></li>
  <li><a href="LAYOUTS.md">Layouts</a></li>
  <li><a href="DAEMON.md">Daemon Mode</a></li>
  <li><a href="GEN_CONFIG.md">Config Generation</a></li>
  <li><a href="PLUGINS.md">Plugins</a></li>
  <li><a href="EFFECTS.md">Effects</a></li>
  <li><a href="EXTENSIONS.md">Extensions</a></li>
  <li><a href="THEMES.md">Themes</a></li>
  <li><a href="WASM.md">WebAssembly Guests</a></li>
  <li><a href="UNINSTALLATION.md">Uninstallation</a></li>
</ul>
