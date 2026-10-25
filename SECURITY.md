# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in **xfetch**, please report it responsibly by contacting:

**Email:** `x@xscriptor.com`

### What to Include

When reporting a security issue, please provide:

1. **Description** — A clear explanation of the vulnerability
2. **Type** — What kind of security issue is it? (e.g., buffer overflow, injection, privilege escalation, etc.)
3. **Steps to Reproduce** — Detailed steps to trigger the vulnerability
4. **Impact** — How severe is the issue? What could an attacker do?
5. **Affected Versions** — Which versions of xfetch are affected?
6. **Proposed Fix** (optional) — If you have a suggestion for how to fix it

### Guidelines

- **Do not** open public GitHub issues for security vulnerabilities
- **Do not** disclose the vulnerability publicly until a fix is released
- **Do** give the maintainers reasonable time to address the issue before public disclosure
- Typically, we aim to respond within **7 days** and release a fix within **30 days** for critical issues

## Security Best Practices for Users

While xfetch is designed to be safe, keep these recommendations in mind:

1. **Keep updated** — Always use the latest version of xfetch
2. **Verify downloads** — Check checksums when downloading binaries
3. **Configuration privacy** — Be careful when sharing custom configs that may contain sensitive paths
4. **Network features** — If using IP detection features, be aware of potential privacy implications
5. **Third-party plugins** — Only use plugins from trusted sources if the plugin system is enabled

## Verifying Release Artifacts

Every release publishes:

- `SHA256SUMS` — checksums for all release archives (published for every release)
- `SHA256SUMS.bundle` — a keyless sigstore (cosign) signature over `SHA256SUMS`
  (published for every release from **v1.0.1** onward)
- GitHub build provenance attestations for each archive (from v1.0.1 onward)

Signing uses GitHub Actions OIDC; there is no long-lived release key.

### Verify the checksums signature

```bash
VERSION=1.0.1   # the release you downloaded
BASE="https://github.com/xfetch-cli/xfetch/releases/download/v${VERSION}"

curl -fsSLO "${BASE}/SHA256SUMS"
curl -fsSLO "${BASE}/SHA256SUMS.bundle"

cosign verify-blob \
  --bundle SHA256SUMS.bundle \
  --certificate-identity "https://github.com/xfetch-cli/xfetch/.github/workflows/release.yml@refs/tags/v${VERSION}" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  SHA256SUMS
```

### Verify an archive against the signed checksums

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

On macOS use `shasum -a 256 -c SHA256SUMS --ignore-missing`; when neither tool
exists, `openssl dgst -sha256 <file>` prints the digest to compare by hand.

### Verify the build provenance attestation

```bash
VERSION=1.0.1
gh attestation verify "xfetch-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
  --repo xfetch-cli/xfetch
```

The expected signing identity is
`https://github.com/xfetch-cli/xfetch/.github/workflows/release.yml@refs/tags/v*`
and the issuer is `https://token.actions.githubusercontent.com`.

### What `install-prebuilt.sh` enforces

| Situation | Behavior |
|-----------|----------|
| `SHA256SUMS` missing or empty | **Abort** — it is mandatory for every prebuilt install |
| Release tag missing/invalid, or asset name not matching the tag version | **Abort** |
| `--version` does not match the release `tag_name` | **Abort** |
| Archive not listed in `SHA256SUMS` | **Abort** |
| Archive digest mismatch | **Abort** |
| No SHA-256 tool (`sha256sum`, `shasum`, `openssl`) | **Abort** (unless `--no-checksum`) |
| v1.0.1+ release, `SHA256SUMS.bundle` missing | **Abort** unless `--allow-unsigned` |
| Signature present but invalid | **Abort** always, including with `--allow-unsigned` |
| `< v1.0.1` release (never signed), bundle missing | Warning, SHA-256 only |
| `cosign` not installed | Warning, SHA-256 only; `--require-signature` makes it fatal |
| `--no-checksum` | **Skips both checks** — dangerous, for trusted/offline sources only |

`--require-signature` and `--no-checksum` are mutually exclusive. The installer
never silently downgrades because a hashing tool is missing, and it verifies the
cosign signature even when no local hashing tool exists (the install still
aborts afterwards if the archive digest cannot be computed).

### Release pipeline safeguards

The release workflow only runs for `v*` tags and refuses to publish unless:

1. the tagged commit is an ancestor of `origin/main` (checked with full history);
2. the tag matches the `version` in `Cargo.toml`;
3. `cargo test --locked` and `cargo audit` pass in a gate job before any build;
4. the archives, `SHA256SUMS` and `SHA256SUMS.bundle` are present, the signature
   verifies against the workflow identity, and `sha256sum -c` passes;
5. only the expected artifact name patterns are uploaded.

The publishing job additionally uses the GitHub environment `release`. The
repository owner must configure required reviewers / deployment protection rules
for that environment in GitHub settings — this is **not** enforceable from
workflow YAML, so the YAML alone does not prove that a human approved a release.

### Known limitations

- `install-prebuilt.ps1` (Windows) verifies `SHA256SUMS` but does **not** yet
  verify `SHA256SUMS.bundle` with cosign; Windows users get checksum-level
  integrity only until that is implemented.
- Releases before v1.0.1 are unsigned; they can only be SHA-256 verified.
- `--no-checksum` and `--allow-unsigned` weaken verification by user choice and
  print prominent warnings.

## Supported Versions

| Version | Status | Support Until |
|---------|--------|----------------|
| latest  | Active | latest        |

## Security Updates

Security fixes will be released as soon as possible. Critical vulnerabilities will receive priority treatment.

---

**Thank you for helping keep xfetch secure!**
