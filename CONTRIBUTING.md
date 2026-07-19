<div align="center">
  <h1> Contributing to xfetch</h1>
  <p>Thank you for your interest in contributing to xfetch!</p>
</div>

<br>

<h2>Code of Conduct</h2>

<p>
  This project is open and welcoming. Be respectful, constructive, and collaborative.
  Harassment, trolling, and personal attacks are not tolerated.
</p>

<h2>How to Contribute</h2>

<ol>
  <li><strong>Fork</strong> the repository on GitHub.</li>
  <li><strong>Clone</strong> your fork locally.</li>
  <li>Create a <strong>feature branch</strong> (<code>git checkout -b feature/my-change</code>).</li>
  <li>Make your changes following the project conventions.</li>
  <li>Run <code>cargo build</code> and <code>cargo test</code> to verify everything works.</li>
  <li><strong>Commit</strong> your changes with a clear message.</li>
  <li><strong>Push</strong> to your fork.</li>
  <li>Open a <strong>Pull Request</strong> from your branch to the main repository.</li>
</ol>

<h2>Contributing Plugins</h2>

<p>
  Plugins are standalone executables that extend xfetch's functionality, but they
  now live in the dedicated <code>plugins</code> repository:
</p>

<pre><code>https://github.com/xfetch-cli/plugins</code></pre>

<p>
  Use that repository for plugin source code, plugin-specific docs, and the plugin
  contribution workflow. The runtime contract used by the core is documented in
  <a href="docs/PLUGINS.md">docs/PLUGINS.md</a>, and the full authoring guide lives in
  <a href="https://github.com/xfetch-cli/plugins/blob/main/docs/README.md">plugins/docs/README.md</a>.
</p>

<p>The plugin authoring checklist includes:</p>

<ul>
  <li>Create a dedicated plugin directory in the plugins repository.</li>
  <li>Use the <code>xfetch-plugin-&lt;name&gt;</code> naming convention.</li>
  <li>Run <code>cargo test --workspace</code> in the plugins repository.</li>
  <li>Document configuration, dependencies, and example output in the plugin README.</li>
</ul>

<h2>Code Standards</h2>

<ul>
  <li>Use <code>cargo build</code> and <code>cargo test</code> before submitting.</li>
  <li>Follow existing code style — no trailing whitespace, consistent indentation.</li>
  <li>Write all code, comments, and documentation in English.</li>
  <li>Keep plugins focused on a single responsibility.</li>
  <li>Minimize dependencies to keep build times fast.</li>
  <li>Do not add network calls or file I/O to animation plugins — they should be pure transformations.</li>
</ul>

<h2>Reporting Issues</h2>

<p>
  Open an issue on GitHub with a clear description of the problem, steps to reproduce,
  and your environment (OS, terminal emulator, xfetch version).
</p>

<h2>License</h2>

<p>
  By contributing, you agree that your contributions will be licensed under the project's
  <a href="LICENSE">LICENSE</a>.
</p>
