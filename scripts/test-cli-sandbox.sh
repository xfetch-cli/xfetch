#!/usr/bin/env bash
# End-to-end CLI sandbox test. Usage: scripts/test-cli-sandbox.sh [xfetch-binary]
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${1:-$SCRIPT_DIR/../target/release/xfetch}"
if [ ! -x "$BIN" ]; then
  echo "xfetch binary not found at $BIN"
  echo "Run: cargo build --release --locked"
  exit 1
fi
SB="$(mktemp -d /tmp/xfetch-cli-sandbox.XXXXXX)"
export HOME="$SB/home"
export XDG_CONFIG_HOME="$SB/config"
export XDG_CACHE_HOME="$SB/cache"
export XDG_STATE_HOME="$SB/state"
OUT="$SB/out.txt"
ERR="$SB/err.txt"
mkdir -p "$HOME" "$XDG_CONFIG_HOME/xfetch" "$XDG_CACHE_HOME" "$XDG_STATE_HOME"

pass=0
fail_count=0
fail() { echo "FAIL: $*"; fail_count=$((fail_count + 1)); }
ok() { pass=$((pass + 1)); }
run() { "$@" >"$OUT" 2>"$ERR" || { fail "command failed: $*"; cat "$OUT" "$ERR"; }; }
expect() {
  local want=$1; shift
  "$@" >"$OUT" 2>"$ERR"
  local got=$?
  if [ "$got" -ne "$want" ]; then
    fail "expected exit $want, got $got: $*"
    cat "$OUT" "$ERR"
  else
    ok
  fi
}
contains() { grep -q -- "$1" "$OUT" || fail "missing '$1' in output of $2"; }
json_check() { python3 -c "$1" <"$OUT" || fail "json check failed: $1"; }

# Basic info and discovery.
run "$BIN" --version
contains "xfetch 1.0.3" 1
run "$BIN" --help
contains "--backup" help
contains "--restore" help
run "$BIN" --list-layouts
contains "right" layouts
run "$BIN" --json --list-layouts
json_check 'import json,sys; v=json.load(sys.stdin); assert "right" in v'
run "$BIN" --list-modules
contains "packages:<manager>" modules
run "$BIN" modules
contains "packages:<manager>" modules
run "$BIN" --json modules
json_check 'import json,sys; v=json.load(sys.stdin); assert any(x["key"]=="os" for x in v)'
run "$BIN" --json --list-modules
json_check 'import json,sys; v=json.load(sys.stdin); assert any(x["key"]=="os" for x in v)'

# Output modes.
run "$BIN" --json
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1 and v["name"]=="xfetch"'
run "$BIN" --json --benchmark
json_check 'import json,sys; v=json.load(sys.stdin); assert "benchmark" in v'
run "$BIN" --get os
[ -n "$(cat "$OUT")" ] || fail "--get os returned no value"
run "$BIN" --get os,cpu
contains "os=" get-os-cpu
contains "cpu=" get-os-cpu
run "$BIN" --json --get os,cpu
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1 and "values" in v and "os" in v["values"]'
run "$BIN" --format 'os={{os}} name={{name}} version={{version}} missing={{nope}}'
contains "os=" format
contains "name=xfetch" format
contains "version=1.0.3" format
printf 'kernel={{kernel}}\n' >"$SB/template.txt"
run "$BIN" --template "$SB/template.txt"
contains "kernel=" template

# Runtime overrides.
run "$BIN" --layout right --json
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1'
run "$BIN" --modules os,kernel --json
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1'
run "$BIN" --color never --modules os,kernel
[ ! -s "$OUT" ] || ! grep -q $'\x1b' "$OUT" || fail "ANSI found with --color never"
run "$BIN" --color always --modules os,kernel
grep -q $'\x1b' "$OUT" || fail "no ANSI with --color always"
run "$BIN" --no-cache --json
run "$BIN" --offline --json
run "$BIN" --quiet --json
run "$BIN" --verbose --get os
grep -q "config:" "$ERR" || fail "verbose diagnostics missing"

# Conflicts and stable errors.
expect 2 "$BIN" --json --format '{{os}}'
expect 2 "$BIN" --get os --format '{{os}}'
expect 2 "$BIN" --layout nope --get os
expect 3 "$BIN" --theme nope --get os
expect 7 "$BIN" --get nope
expect 4 "$BIN" --offline plugin search logo
expect 6 "$BIN" --offline update --check

# Configuration commands and simple backup/restore.
CONFIG="$XDG_CONFIG_HOME/xfetch/config.jsonc"
printf '{\n  // keep me\n  "layout": "section",\n  "modules": ["os", "kernel"]\n}\n' >"$CONFIG"
run "$BIN" config validate
contains "Config OK" validate
run "$BIN" --json config validate
json_check 'import json,sys; v=json.load(sys.stdin); assert v["valid"] is True'
run "$BIN" config print --raw
contains "// keep me" print-raw
run "$BIN" config print
contains '"layout"' print
run "$BIN" --json config print
json_check 'import json,sys; v=json.load(sys.stdin); assert "layout" in v'
run "$BIN" config get layout
contains "section" get-layout
run "$BIN" --json config get modules
json_check 'import json,sys; v=json.load(sys.stdin); assert isinstance(v,list) and "os" in v'
run "$BIN" --quiet config set layout right
[ -s "$OUT" ] && fail "quiet config set printed output"
grep -q '"layout": "right"' "$CONFIG" || fail "config set did not write layout right"
[ "$(grep -c '// keep me' "$CONFIG")" -eq 1 ] || fail "comment lost"
run "$BIN" --json config set colors.cpu red
json_check 'import json,sys; v=json.load(sys.stdin); assert v["key"]=="colors.cpu"'

# Backup/restore: simple config.
run "$BIN" --json --backup
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1 and v["path"].endswith(".json")'
BACKUP_SIMPLE=$(python3 -c 'import json,sys; print(json.load(open("/dev/stdin"))["path"])' <"$OUT")
printf '{ "layout": "minimal" }\n' >"$CONFIG"
run "$BIN" --json --restore
json_check 'import json,sys; v=json.load(sys.stdin); assert v["restored"]'
grep -q '"layout": "right"' "$CONFIG" || fail "restore did not restore layout right"
run "$BIN" config backup --output "$SB/backups"
mkdir -p "$SB/backups" 2>/dev/null || true
[ -f "$BACKUP_SIMPLE" ] || fail "simple backup file missing"
printf '{ "layout": "tree" }\n' >"$CONFIG"
LATEST_DIR="$XDG_CONFIG_HOME/xfetch/backups"
run "$BIN" config restore --from "$LATEST_DIR"
grep -q '"layout": "right"' "$CONFIG" || fail "restore from dir did not restore layout right"
printf '{ "layout": "tree" }\n' >"$CONFIG"
run "$BIN" config restore --dry-run
contains "Would restore" dry-run
grep -q '"layout": "tree"' "$CONFIG" || fail "dry-run wrote to the config"

# Themes.
THEMES="$XDG_CONFIG_HOME/xfetch/themes"
mkdir -p "$THEMES"
printf '{ "layout": "compact", "colors": { "os": "Cyan" } }\n' >"$THEMES/nord.jsonc"
run "$BIN" --json theme list
json_check 'import json,sys; v=json.load(sys.stdin); assert any(t["name"]=="nord" for t in v["themes"])'
run "$BIN" --json theme set nord
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="set"'
run "$BIN" --theme nord --json
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1'
run "$BIN" --theme nord --json config print
json_check 'import json,sys; v=json.load(sys.stdin); assert v["layout"]=="compact"'
run "$BIN" --layout right --json config print
json_check 'import json,sys; v=json.load(sys.stdin); assert v["layout"]=="right"'
run "$BIN" --json theme export exported
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="exported"'
run "$BIN" --json theme remove exported
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="removed"'

# Doctor.
run "$BIN" doctor
contains "checks:" doctor
run "$BIN" --json doctor
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1 and v["checks"]'
run "$BIN" --quiet doctor
! grep -q "\[environment\]" "$OUT" || fail "quiet doctor printed OK checks"


# Diagnostics commands and structured logging.
run "$BIN" paths
contains "config" paths
run "$BIN" --json paths
json_check 'import json,sys; v=json.load(sys.stdin); assert v["config"] and v["config_dir"]'
run "$BIN" env
contains "platform" env
run "$BIN" --json env
json_check 'import json,sys; v=json.load(sys.stdin); assert v["build"] and v["settings"]'
run "$BIN" build-info
contains "wasm support" build-info
run "$BIN" --json build-info
json_check 'import json,sys; v=json.load(sys.stdin); assert v["features"]["encryption"] is True'
run "$BIN" --json doctor --bench
json_check 'import json,sys; v=json.load(sys.stdin); assert "benchmark" in v'
run "$BIN" doctor --image
run "$BIN" --log-json --log-level debug --verbose --get os
python3 - "$ERR" <<'PY'
import json, sys
lines=[line for line in open(sys.argv[1]) if line.strip()]
assert lines, "no structured logs"
for line in lines:
    data=json.loads(line)
    assert data["level"] == "debug"
    assert data["target"] == "xfetch"
PY
[ $? -eq 0 ] || fail "structured log validation failed"
run "$BIN" --log-level off --verbose --get os
[ ! -s "$ERR" ] || fail "log level off still wrote logs"
run "$BIN" help plugin install
contains "Install an addon" help-plugin-install

# Shell integration.
for shell in bash zsh fish elvish powershell; do
  run "$BIN" completions "$shell"
  [ -s "$OUT" ] || fail "empty completions for $shell"
done
run "$BIN" man
contains ".TH xfetch" man
run "$BIN" man --output "$SB/xfetch.1"
[ -s "$SB/xfetch.1" ] || fail "man output file missing"

# Wasm tooling (no-op/error paths).
run "$BIN" wasm wit
contains "xfetch:runtime" wasm-wit
printf 'not wasm\n' >"$SB/not-wasm.bin"
expect 5 "$BIN" wasm inspect "$SB/not-wasm.bin"
expect 5 "$BIN" wasm run "$SB/not-wasm.bin"

# Addon commands.
run "$BIN" --json addon plugin list
json_check 'import json,sys; v=json.load(sys.stdin); assert v["schema_version"]==1 and isinstance(v["addons"],list)'
run "$BIN" --json plugin list
json_check 'import json,sys; v=json.load(sys.stdin); assert isinstance(v["addons"],list)'
expect 4 "$BIN" --offline addon plugin search logo
expect 4 "$BIN" --offline --json addon plugin install --all
expect 5 "$BIN" --json addon plugin rollback x
expect 5 "$BIN" --json addon plugin remove x

# JSON addon rollback/remove against a fake installed native artifact.
PLUGINS="$XDG_CONFIG_HOME/xfetch/plugins"
mkdir -p "$PLUGINS"
printf 'new' >"$PLUGINS/xfetch-plugin-test"
printf 'old' >"$PLUGINS/xfetch-plugin-test.bak"
run "$BIN" --json addon plugin rollback test
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="rolled-back"'
[ "$(cat "$PLUGINS/xfetch-plugin-test")" = "old" ] || fail "rollback did not restore the backup"
run "$BIN" --json addon plugin remove test
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="removed"'
[ ! -e "$PLUGINS/xfetch-plugin-test" ] || fail "remove did not delete the artifact"

# Daemon/cache no-op paths.
run "$BIN" --json --daemon-stop
json_check 'import json,sys; v=json.load(sys.stdin); assert "stopped" in v'
run "$BIN" --json --daemon-live-stop
json_check 'import json,sys; v=json.load(sys.stdin); assert "stopped" in v'
run "$BIN" --json --clean-cache
json_check 'import json,sys; v=json.load(sys.stdin); assert v["cleaned"] is True'

# Config-roulette: routes + theme + fake extension.
ROULETTE="$XDG_CONFIG_HOME/xfetch/roulette"
mkdir -p "$ROULETTE/a" "$ROULETTE/b"
printf '{ "layout": "compact", "colors": { "os": "Cyan" } }\n' >"$THEMES/theme.jsonc"
printf '{ "layout": "compact", "theme": "theme", "modules": ["os"] }\n' >"$ROULETTE/a/config.jsonc"
printf '{ "layout": "tree", "modules": ["kernel"] }\n' >"$ROULETTE/b/config.jsonc"
printf '[{"_name":"a","path":"%s/a/config.jsonc"},{"_name":"b","path":"%s/b/config.jsonc"}]\n' "$ROULETTE" "$ROULETTE" >"$ROULETTE/routes.json"
printf '{\n  "config_providers": [\n    { "extension": "config-roulette", "args": { "routes": "%s/routes.json" } }\n  ]\n}\n' "$ROULETTE" >"$XDG_CONFIG_HOME/xfetch/config.jsonc"
EXT="$XDG_CONFIG_HOME/xfetch/extensions"
mkdir -p "$EXT"
cat >"$EXT/xfetch-extension-config-roulette" <<'PY'
#!/usr/bin/env python3
import json, os, sys
request = json.load(sys.stdin)
args = request.get("args") or {}
routes = os.path.expanduser(args.get("routes", "~/.config/xfetch/routes.json"))
with open(routes, encoding="utf-8") as handle:
    routes_data = json.load(handle)
choice = routes_data[0]
path = os.path.expanduser(choice["path"])
if not os.path.isabs(path):
    path = os.path.join(os.path.dirname(routes), path)
with open(path, encoding="utf-8") as handle:
    config = json.load(handle)
json.dump({"config": config}, sys.stdout)
PY
chmod +x "$EXT/xfetch-extension-config-roulette"
run "$BIN" --json config print
json_check 'import json,sys; v=json.load(sys.stdin); assert v["layout"]=="compact"'
run "$BIN" config backup --output "$SB/roulette-backup.json"
python3 - "$SB/roulette-backup.json" <<'PY' >"$SB/roulette-roles.txt"
import json, sys
b=json.load(open(sys.argv[1]))
roles=[f["role"] for f in b["files"]]
print(",".join(roles))
assert "config" in roles
assert "routes" in roles
assert "theme" in roles
assert roles.count("route-config") == 2
print("ROULETTE_OK")
PY
grep -q "ROULETTE_OK" "$SB/roulette-roles.txt" || fail "roulette backup roles check failed"
# Restore the roulette bundle after deleting its files.
rm -f "$XDG_CONFIG_HOME/xfetch/config.jsonc" "$ROULETTE/routes.json" "$ROULETTE/a/config.jsonc" "$ROULETTE/b/config.jsonc" "$THEMES/theme.jsonc"
run "$BIN" config restore --from "$SB/roulette-backup.json"
[ -f "$XDG_CONFIG_HOME/xfetch/config.jsonc" ] || fail "roulette config not restored"
[ -f "$ROULETTE/routes.json" ] || fail "roulette routes not restored"
[ -f "$ROULETTE/a/config.jsonc" ] || fail "route a not restored"
[ -f "$ROULETTE/b/config.jsonc" ] || fail "route b not restored"
[ -f "$THEMES/theme.jsonc" ] || fail "theme not restored"


# Template conditionals and loops.
run "$BIN" --format '{{#if kernel}}kernel-ok{{else}}no-kernel{{/if}} {{#unless nope}}unless-ok{{/unless}} {{#each gpu}}[{{this}}]{{else}}none{{/each}}'
contains "kernel-ok" template-if
contains "unless-ok" template-unless

# Config editing: array set, unset, edit and reset.
EDIT_CFG="$SB/edit-config.jsonc"
printf '{ "layout": "section" }\n' >"$EDIT_CFG"
run "$BIN" --config "$EDIT_CFG" config set modules.0 os
run "$BIN" --config "$EDIT_CFG" config set modules.1 kernel
grep -q '"kernel"' "$EDIT_CFG" || fail "config set modules.1 failed"
run "$BIN" --config "$EDIT_CFG" config unset modules.1
! grep -q '"kernel"' "$EDIT_CFG" || fail "config unset modules.1 failed"
cat >"$SB/editor.sh" <<'SH'
#!/bin/sh
printf '{ "layout": "minimal" }\n' > "$1"
SH
chmod +x "$SB/editor.sh"
env EDITOR="$SB/editor.sh" "$BIN" --config "$EDIT_CFG" config edit >"$OUT" 2>"$ERR" || fail "config edit failed"
grep -q '"minimal"' "$EDIT_CFG" || fail "config edit did not apply the editor output"
run "$BIN" --config "$EDIT_CFG" config reset --yes
grep -q 'xfetch configuration' "$EDIT_CFG" || fail "config reset did not write defaults"
run "$BIN" --json config backups
json_check 'import json,sys; v=json.load(sys.stdin); assert v["backups"]'
run "$BIN" --json --backup
run "$BIN" --json config prune --keep 0
json_check 'import json,sys; v=json.load(sys.stdin); assert v["deleted"]'

# Backup compression, encryption, addon assets and root remapping.
ROOT_CFG="$SB/root/xfetch/config.jsonc"
mkdir -p "$SB/root/xfetch/themes" "$SB/compressed"
printf '{ "layout": "compact" }\n' >"$SB/root/xfetch/themes/nord.jsonc"
printf '{ "theme": "%s/root/xfetch/themes/nord.jsonc" }\n' "$SB" >"$ROOT_CFG"
run "$BIN" --config "$ROOT_CFG" config backup --output "$SB/compressed" --compress
ls "$SB/compressed"/*.json.gz >/dev/null 2>&1 || fail "compressed backup missing"
printf 'secret' >"$SB/pass.txt"
run "$BIN" --config "$ROOT_CFG" config backup --output "$SB/encrypted.enc" --encrypt --passphrase-file "$SB/pass.txt"
[ -s "$SB/encrypted.enc" ] || fail "encrypted backup missing"
printf '{ "layout": "tree" }\n' >"$ROOT_CFG"
rm -f "$SB/root/xfetch/themes/nord.jsonc"
run "$BIN" --config "$ROOT_CFG" config restore --from "$SB/encrypted.enc" --passphrase-file "$SB/pass.txt"
grep -q 'theme' "$ROOT_CFG" || fail "encrypted restore did not restore the config"
grep -q 'compact' "$SB/root/xfetch/themes/nord.jsonc" || fail "encrypted restore did not restore the theme"
PLUGIN_ARTIFACT="$XDG_CONFIG_HOME/xfetch/plugins/xfetch-plugin-json-test"
printf 'artifact' >"$PLUGIN_ARTIFACT"
run "$BIN" --config "$ROOT_CFG" config backup --output "$SB/with-addons.json" --with-addons
python3 - "$SB/with-addons.json" <<'PY'
import json, sys
b=json.load(open(sys.argv[1]))
roles=[f["role"] for f in b["files"]]
assert "addon-artifact" in roles, roles
print("ADDON_ASSET_OK")
PY
run "$BIN" --config "$ROOT_CFG" config backup --output "$SB/root-backup.json"
rm -rf "$SB/root"
run "$BIN" config restore --from "$SB/root-backup.json" --root "$SB/root-restored"
[ -f "$SB/root-restored/config.jsonc" ] || fail "root remapped config missing"
[ -f "$SB/root-restored/themes/nord.jsonc" ] || fail "root remapped theme missing"

# Doctor --fix generates a missing config.
FIX_XDG="$SB/fix-config"
env XDG_CONFIG_HOME="$FIX_XDG" HOME="$SB/fix-home" XDG_CACHE_HOME="$SB/fix-cache" "$BIN" --json doctor --fix >"$OUT" 2>"$ERR" || fail "doctor --fix failed"
json_check 'import json,sys; v=json.load(sys.stdin); assert v["fixes"]'
[ -f "$FIX_XDG/xfetch/config.jsonc" ] || fail "doctor --fix did not generate a config"

# Schema output.
run "$BIN" --json schema fetch
json_check 'import json,sys; v=json.load(sys.stdin); assert v["title"]=="xfetch fetch output"'

# Core binary rollback.
RB="$SB/rollback-bin"
mkdir -p "$RB"
printf 'new' >"$RB/xfetch"
printf 'old' >"$RB/xfetch.bak"
run "$BIN" --json update --rollback --bin-dir "$RB"
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="rolled-back"'
[ "$(cat "$RB/xfetch")" = "old" ] || fail "update rollback failed"

# Remote theme and logo catalogs via a local HTTP server.
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ -d "$REPO_ROOT/../logos" ] && [ -d "$REPO_ROOT/../themes" ]; then
  HTTP_ROOT="$SB/http-root"
  mkdir -p "$HTTP_ROOT"
  ln -s "$REPO_ROOT/../logos" "$HTTP_ROOT/logos"
  ln -s "$REPO_ROOT/../themes" "$HTTP_ROOT/themes"
  python3 - "$HTTP_ROOT/themes/index.json" "$HTTP_ROOT/themes/index.local.json" <<'PY'
import json, sys
src, dst = sys.argv[1], sys.argv[2]
data = json.load(open(src))
base = "http://127.0.0.1:8765/themes"
for entry in data.get("themes", []):
    entry["source"] = entry["source"].replace("https://raw.githubusercontent.com/xfetch-cli/themes/main", base)
json.dump(data, open(dst, "w"))
PY
  python3 -m http.server 8765 --directory "$HTTP_ROOT" >"$SB/http.log" 2>&1 &
  HTTP_PID=$!
  trap 'kill "$HTTP_PID" 2>/dev/null || true' EXIT
  export XFETCH_LOGOS_URL="http://127.0.0.1:8765/logos"
  export XFETCH_THEME_INDEX="http://127.0.0.1:8765/themes/index.local.json"
  sleep 1
  run "$BIN" --json logo list
  json_check 'import json,sys; v=json.load(sys.stdin); assert v["logos"]'
  run "$BIN" logo preview arch
  [ -s "$OUT" ] || fail "logo preview was empty"
  expect 4 "$BIN" --offline logo list
  run "$BIN" --json theme search nord
  json_check 'import json,sys; v=json.load(sys.stdin); assert any(t["name"]=="nord" for t in v["themes"])'
  run "$BIN" --json theme install nord --force
  json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="installed"'
  run "$BIN" --json theme info nord
  json_check 'import json,sys; v=json.load(sys.stdin); assert v["theme"]["name"]=="nord"'
  run "$BIN" --json theme update --all --check
  json_check 'import json,sys; v=json.load(sys.stdin); assert "themes" in v'
  run "$BIN" --json theme remove nord
  json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="removed"'
  expect 4 "$BIN" --offline theme search nord
fi

# Conflicts.
expect 2 "$BIN" --backup --restore
expect 2 "$BIN" --backup modules

# Self-uninstall a copied binary.
UNINSTALL_BIN="$SB/uninstall-bin"
mkdir -p "$UNINSTALL_BIN"
cp "$BIN" "$UNINSTALL_BIN/xfetch"
env XDG_CONFIG_HOME="$SB/uninstall-config" HOME="$SB/uninstall-home" XDG_CACHE_HOME="$SB/uninstall-cache" \
  "$UNINSTALL_BIN/xfetch" --json self-uninstall --purge --yes >"$OUT" 2>"$ERR" || fail "self-uninstall failed"
json_check 'import json,sys; v=json.load(sys.stdin); assert v["status"]=="uninstalled"'
[ ! -e "$UNINSTALL_BIN/xfetch" ] || fail "self-uninstall did not remove the binary"

# Conflicts.
expect 2 "$BIN" --backup --restore
expect 2 "$BIN" --backup modules

echo
echo "SANDBOX=$SB"
echo "PASS=$pass FAIL=$fail_count"
[ "$fail_count" -eq 0 ]
