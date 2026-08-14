<h1>Daemon Mode</h1>

<p>
  Daemon mode pins an animated fetch at the top of the terminal and keeps looping
  it in the background, so the shell prompt stays usable below it — without splits
  or an extra terminal multiplexer.
</p>

<h2>How it works</h2>

<p>
  When daemon mode is active, xfetch forks to the background: the parent writes its
  PID to <code>~/.config/xfetch/daemon.pid</code> and exits immediately, so the shell
  prompt returns instantly. The child keeps rendering the animation loop pinned at
  the top of the terminal, reserving the top rows with a terminal scroll region
  (<code>DECSTBM</code>) and drawing each frame with absolute cursor positioning.
</p>

<p>No extra shell configuration is required — everything activates from the JSON.</p>

<h2>Activation</h2>

<p>From the CLI:</p>

<pre><code class="language-bash">xfetch --daemon</code></pre>

<p>Or from the config file:</p>

<pre><code class="language-jsonc">{
    "daemon": true,
    "logo_animation": {
        "plugin": "animate-logo",
        "style": "frame",
        "fps": 6,
        "frames_path": "~/.config/xfetch/logos/fox.txt"
    }
}</code></pre>

<p>
  The <code>--daemon</code> CLI flag overrides the <code>daemon</code> config value.
</p>

<h2>Stopping the daemon</h2>

<pre><code class="language-bash">xfetch --daemon-stop</code></pre>

<p>
  This reads the PID from <code>~/.config/xfetch/daemon.pid</code>, verifies it is an
  xfetch process, sends <code>SIGTERM</code>, and restores the terminal (cursor shown,
  scroll region reset, PID file removed).
</p>

<h2>Configuration</h2>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>daemon</code></td><td>boolean</td><td><code>false</code></td>
      <td>Enable daemon mode.</td>
    </tr>
    <tr>
      <td><code>daemon_min_rows</code></td><td>number</td><td>6</td>
      <td>Minimum number of terminal rows left free below the pinned block for command output.</td>
    </tr>
  </tbody>
</table>

<h2>Behavior details</h2>

<ul>
  <li>The animation only runs in TTY (interactive) terminals; on redirects or pipes the static logo is shown.</li>
  <li>Requires a <code>logo_animation</code> block with a plugin (e.g. <code>animate-logo</code>); without one, daemon mode does nothing.</li>
  <li>The pinned block is redrawn as a single atomic write per frame, the cursor is restored after every frame (typed input is never disturbed), and the scroll region is re-asserted every frame.</li>
  <li>Terminal resizes (<code>SIGWINCH</code>) are detected and the geometry is recomputed automatically.</li>
  <li>In daemon mode the animation loops indefinitely: <code>duration_ms</code> and <code>loop</code> in <code>logo_animation</code> are ignored. For a finite animation that stops on its own, use <code>"daemon": false</code>.</li>
</ul>

<h2>Terminal height</h2>

<p>
  Tall figures (e.g. <code>cat</code> ~29 rows, <code>matrix</code> ~24 rows) take up
  almost a whole 30-row terminal and leave little room for command output. The daemon
  reserves the top rows for the logo and keeps command output in the remaining height
  below it. Medium-height figures (~13-17 rows) are recommended for standard terminals.
</p>

<h2>Notes &amp; troubleshooting</h2>

<ul>
  <li><code>~/.config/xfetch/daemon.rows</code> stores the pinned block height, available for optional shell integration.</li>
  <li>If the terminal is closed while the daemon runs, the daemon exits on its own (the output device disappears).</li>
  <li>If the PID file is missing or stale, <code>xfetch --daemon-stop</code> reports "No daemon running" and cleans up the stale file.</li>
  <li>Orphaned daemons can accumulate if <code>--daemon-stop</code> is not used (e.g. after killing the terminal abruptly); clean them up with <code>pkill xfetch</code>.</li>
</ul>
