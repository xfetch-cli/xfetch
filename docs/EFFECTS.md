<h1>Effects</h1>

<p>
  <strong>Effects</strong> are installable intro animations: they change the way the info
  appears when xfetch starts. The core renders the module lines, hands them to an effect
  binary (<code>xfetch-effect-&lt;name&gt;</code>), and plays the returned frames before
  settling on the final content. Example: the <code>decrypt</code> effect reveals each line
  from scrambled glyphs.
</p>

<p>
  Effects are <strong>opt-in</strong> — without the binary installed, xfetch renders normally.
</p>

<h2>Install</h2>

<p>From the effects repository (default: <a href="https://github.com/xfetch-cli/effects">xfetch-cli/effects</a>):</p>

<pre><code class="language-bash">xfetch effects install decrypt</code></pre>

<p>Or from a local path:</p>

<pre><code class="language-bash">xfetch effects install ./effects/decrypt</code></pre>

<p>Manage installed effects:</p>

<pre><code class="language-bash">xfetch effects list      # list installed effects
xfetch effects remove decrypt</code></pre>

<p>Binaries are stored in <code>~/.config/xfetch/effects/</code>. Use
<code>--repo &lt;url&gt;</code> (or the <code>XFETCH_EFFECT_REPO</code> env var) to install
from a fork or mirror.</p>

<h2>Configuration</h2>

<pre><code class="language-jsonc">{
    "effects": [
        { "plugin": "glitch", "duration_ms": 700, "fps": 30 },
        { "plugin": "decrypt", "duration_ms": 1500, "fps": 30 }
    ]
}</code></pre>

<p>
  The <code>effects</code> value accepts a <strong>single object or a list</strong>; a list
  plays the effects in sequence. Each effect receives the same rendered lines and settles
  on them, so chaining glitch → decrypt flows seamlessly into the final fetch.
</p>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr><td><code>plugin</code></td><td>string</td><td>—</td><td>Effect name (binary <code>xfetch-effect-&lt;name&gt;</code>).</td></tr>
    <tr><td><code>style</code></td><td>string</td><td>none</td><td>Effect-specific style selector, passed to the effect.</td></tr>
    <tr><td><code>duration_ms</code></td><td>number</td><td>effect default</td><td>Total animation length in milliseconds.</td></tr>
    <tr><td><code>fps</code></td><td>number</td><td>effect default</td><td>Frames per second.</td></tr>
    <tr><td><code>args</code></td><td>object</td><td>none</td><td>Free-form parameters passed to the effect.</td></tr>
    <tr><td><code>timeout_secs</code></td><td>number</td><td>none</td><td>Safety net: kills the effect process if it runs longer.</td></tr>
  </tbody>
</table>

<h2>Writing an effect</h2>

<p>
  Effects speak the <code>xfetch-effect-api</code> protocol (see
  <a href="https://github.com/xfetch-cli/api">xfetch-cli/api</a>): read an
  <code>EffectRequest</code> from stdin (the rendered lines plus
  <code>style</code>/<code>duration_ms</code>/<code>fps</code>/<code>args</code>) and write an
  <code>EffectResponse</code> with a non-empty list of <code>{ delay_ms, lines }</code> frames.
  The last frame should reach the final content.
</p>

<p>
  Effect implementations live in <a href="https://github.com/xfetch-cli/effects">xfetch-cli/effects</a>
  (one crate per effect, binary <code>xfetch-effect-&lt;name&gt;</code>).
</p>
