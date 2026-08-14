<h1>Plugins</h1>

<p>
  xfetch supports external plugins that run as separate executables. The core binary
  discovers a plugin, sends a JSON request on stdin, and reads a JSON response on
  stdout. The runtime stays in the core repository, while official plugin implementations
  live in the dedicated <strong>plugins</strong> repository.
</p>

<h2>Official Plugin Repository</h2>

<p>
  Official plugins and the full authoring guide are maintained at:
</p>

<pre><code>https://github.com/xfetch-cli/plugins</code></pre>

<p>
  Use that repository for plugin source code, plugin-specific documentation, and
  contribution guidelines.
</p>

<h2>Installation Model</h2>

<p>
  End users should install official plugins by name from the remote repository:
</p>

<pre><code class="language-bash">xfetch plugin install animate-logo</code></pre>

<p>
  The core downloads the plugin source from the official remote, builds it, and installs
  the resulting binary into the xfetch plugin directory.
</p>

<h2>Configuration</h2>

<p>
  Plugins are configured in the main config file. The <code>plugin</code> value can be a
  short name or a full path to an executable.
</p>

<pre><code class="language-jsonc">{
  "logo_animation": {
    "plugin": "animate-logo",
    "style": "frame",
    "fps": 6,
    "duration_ms": 8000,
    "loop": true,
    "frames_path": "~/.config/xfetch/logos/fox.txt"
  }
}
</code></pre>

<p>
  For the <code>frame</code> style, <code>frames_path</code> can be a single file whose
  frames are separated by a line containing <code>===</code>, or an array of files (one
  per frame). Other <code>style</code> values include <code>sweep</code> (default),
  <code>wave</code>, <code>rainbow</code>, <code>sparkle</code>, <code>breathing</code>,
  and <code>none</code>.
</p>

<blockquote>
  <strong>Note:</strong> <code>duration_ms</code> and <code>loop</code> only apply to the
  one-shot animation (daemon mode off). With <code>"daemon": true</code> the animation
  loops indefinitely and these fields are ignored. See <a href="DAEMON.md">DAEMON.md</a>.
</blockquote>

<h2>Protocol</h2>

<p>
  xfetch communicates with plugins using JSON over stdin/stdout. The request includes
  the plugin kind plus any plugin-specific arguments. The response returns either
  rendered lines or animation frames.
</p>

<pre><code class="language-json">{
  "version": 1,
  "kind": "logo_animation",
  "lines": ["__  __", " \\ \\/ /"],
  "args": {
    "fps": 12,
    "duration_ms": 1200,
    "loop": false
  }
}
</code></pre>

<pre><code class="language-json">{
  "frames": [
    {
      "delay_ms": 80,
      "lines": ["__  __", " \\ \\/ /"]
    }
  ]
}
</code></pre>

<h2>Plugin Catalog &amp; Docs</h2>

<p>
  All official plugins, their documentation, and the development guide are maintained in the
  <a href="https://github.com/xfetch-cli/plugins">xfetch-cli/plugins</a> repository:
</p>

<ul>
  <li>Plugin catalog &amp; reference: <a href="https://github.com/xfetch-cli/plugins">github.com/xfetch-cli/plugins</a></li>
  <li>Plugin development guide: <a href="https://github.com/xfetch-cli/plugins/blob/main/docs/README.md">docs/README.md</a></li>
  <li>Shared plugin API crate: <a href="https://github.com/xfetch-cli/api">github.com/xfetch-cli/api</a></li>
</ul>
