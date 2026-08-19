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

<h2>Live Stats Daemon</h2>

<p>
  The <strong>live stats daemon</strong> (<code>daemon_live</code>) is a sibling of the
  animated-logo daemon: it pins the fetch block at the top of the terminal and
  <strong>re-probes a lightweight subset of modules every few seconds</strong>, re-rendering
  the block with fresh values (cpu, memory, swap, disks, battery, uptime, datetime).
  Your fetch stops being a static snapshot and becomes a live panel — think
  "conky pinned at the top", not an interactive btop.
</p>

<p>
  The existing animated daemon is untouched: the two modes are independent and share
  the same pinning/scroll-region machinery.
</p>

<h3>Activation</h3>

<pre><code class="language-jsonc">{
    "daemon_live": true,
    "daemon_live_refresh": 2
}</code></pre>

<p>Activation is config-only; the terminal flags only disable/stop/force:</p>

<pre><code class="language-bash">xfetch --no-daemon-live       # disable even if the config enables it
xfetch --daemon-live-stop     # stop the running live daemon
xfetch --daemon-live-reload   # force hot reload (same as "daemon_live_reload": true)</code></pre>

<h3>Configuration</h3>

<table>
  <thead>
    <tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr>
  </thead>
  <tbody>
    <tr>
      <td><code>daemon_live</code></td><td>boolean</td><td><code>false</code></td>
      <td>Enable the live stats daemon.</td>
    </tr>
    <tr>
      <td><code>daemon_live_refresh</code></td><td>number</td><td>per-platform</td>
      <td>Seconds between refreshes. Defaults to the platform policy (<code>platform/&lt;os&gt;/live.rs</code>): Linux 2, macOS 3, Windows 5.</td>
    </tr>
    <tr>
      <td><code>daemon_live_modules</code></td><td>array</td><td>per-platform</td>
      <td>Modules shown (and refreshed). Defaults to the platform's live set: Linux/macOS <code>cpu, memory, swap, disks, battery, uptime, datetime</code>; Windows excludes <code>battery</code> (it spawns <code>wmic</code>/PowerShell every tick) unless you add it back.</td>
    </tr>
    <tr>
      <td><code>daemon_live_reload</code></td><td>boolean</td><td><code>false</code></td>
      <td>Hot reload: watch the config file (and the active theme) and re-apply changes — modules, colors, icons, layout, logo, refresh cadence — without restarting the daemon. Equivalent CLI flag: <code>--daemon-live-reload</code>.</td>
    </tr>
  </tbody>
</table>

<h3>Behavior details</h3>

<ul>
  <li>If <code>logo_animation</code> is configured, the logo keeps animating while the content refreshes live; otherwise the logo is static.</li>
  <li>Only the modules in <code>daemon_live_modules</code> are probed on each tick — heavy work (packages, public IP, plugins) is never re-run.</li>
  <li>The PID file is <code>~/.config/xfetch/daemon_live.pid</code>, separate from the animated daemon's; <code>--daemon-live-stop</code> targets it.</li>
  <li>Resizes are handled like the animated daemon; <code>daemon_min_rows</code> applies to both.</li>
  <li>Hot reload checks file mtimes (config + theme) on a light poll; edits are picked up within ~100 ms. Re-applying also re-runs config providers (extensions) and, when the animation settings changed, re-spawns the logo plugin. To stop the daemon, still use <code>--daemon-live-stop</code> — setting <code>daemon_live</code> back to <code>false</code> does not stop a running daemon.</li>
</ul>

