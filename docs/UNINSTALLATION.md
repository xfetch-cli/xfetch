<h1>Uninstalling xfetch</h1>

<p>
  We are sorry to see you go! If you encountered any issues, please feel free to open an issue on our GitHub repository.
</p>

<h2>Quick Uninstall</h2>

<p>
  You can uninstall xfetch by running the following command in your terminal:
</p>

<pre><code class="language-bash">curl -fsSL https://raw.githubusercontent.com/xfetch-cli/xfetch/main/uninstall.sh | bash</code></pre>

<h2>Manual Uninstall</h2>

<p>
  If you prefer to uninstall manually, you can delete the following files and directories:
</p>

<ol>
  <li>
    <strong>Binary</strong>: Remove the executable.
    <pre><code class="language-bash">rm ~/.local/bin/xfetch</code></pre>
  </li>
  <li>
    <strong>Configuration</strong>: Remove the configuration directory.
    <pre><code class="language-bash">rm -rf ~/.config/xfetch</code></pre>
  </li>
  <li>
    <strong>Shell Config</strong>: Open your shell configuration file (<code>~/.bashrc</code>, <code>~/.zshrc</code>, or <code>~/.bash_profile</code>) and remove the lines added by the installer:
    <pre><code class="language-bash"># xfetch path
export PATH="$HOME/.local/bin:$PATH"</code></pre>
  </li>
</ol>
