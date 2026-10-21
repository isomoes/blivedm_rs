# Release prompt

Releases are driven by pushing a `vX.Y.Z` tag; `.github/workflows/release.yml` then
verifies `Cargo.toml`'s version matches the tag, publishes to crates.io via Trusted
Publishing, and creates a GitHub Release from `git log` (format shown in
[v0.5.7](https://github.com/isomoes/blivedm_rs/releases/tag/v0.5.7)). `CHANGELOG.md` is
for humans, not the release body. There are no binary assets; crates.io is the artifact.

Let `X.Y.Z` be the new version (feat → minor, fix/docs/chore → patch, breaking → major).

## Steps

1. **Start clean.**

   ```bash
   git status --short                 # must be empty
   git switch master
   git pull --ff-only origin master
   git log $(git describe --tags --abbrev=0)..HEAD --oneline
   ```

2. **Update `CHANGELOG.md`.** Keep an empty `## [Unreleased]` at the top, add
   `## [X.Y.Z] - YYYY-MM-DD` below it, and use Keep a Changelog groups (`Added`,
   `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`) with user-facing bullets.
   Match the existing 0.5.x style; no commit hashes.

3. **Bump the version** in `Cargo.toml`, then refresh `Cargo.lock`:

   ```bash
   cargo check
   git diff Cargo.lock                # only blivedm's version should change
   ```

4. **Validate.**

   ```bash
   cargo test --locked
   cargo build --locked
   git diff --check
   ```

5. **Commit and tag.**

   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "release vX.Y.Z"
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   ```

6. **Push the commit, then the tag.**

   ```bash
   git push origin master
   git push origin vX.Y.Z
   ```

   The tag push starts the workflow. Never move or force-push a pushed tag.

7. **Verify.**

   ```bash
   gh run list --workflow release.yml --limit 1
   gh release view vX.Y.Z
   cargo search blivedm --limit 1
   ```

   Expect a green run, `Release vX.Y.Z` with the v0.5.7 body layout, and the crate at
   `X.Y.Z`.

8. **Report** the version, commit hash, tag, workflow run URL, and release URL.

## On failure

- **Version/tag mismatch:** delete the local tag, fix the version, re-tag, push again.
- **Before `cargo publish` succeeds:** fix and `gh run rerun <run-id> --failed`.
- **After crates.io accepted `X.Y.Z`:** do not republish it. Create the missing release
  manually:

  ```bash
  PREV=$(git describe --tags --abbrev=0 vX.Y.Z^)
  {
    echo "## 📦 Published to crates.io"; echo
    echo "Install with:"; echo '```bash'
    echo "cargo install --locked blivedm"; echo '```'; echo
    echo "## 📝 Changes since $PREV"; echo
    git log --pretty=format:'* %s (@%an) [%h](https://github.com/isomoes/blivedm_rs/commit/%H)' "$PREV..vX.Y.Z"; echo; echo
    echo "## 📚 Documentation"
    echo "- [crates.io](https://crates.io/crates/blivedm)"
    echo "- [GitHub Repository](https://github.com/isomoes/blivedm_rs)"
    echo "- [README](https://github.com/isomoes/blivedm_rs/blob/master/README.md)"
  } > /tmp/release-notes.md
  gh release create vX.Y.Z --title "Release vX.Y.Z" --notes-file /tmp/release-notes.md
  ```

  Ship any changelog correction as `X.Y.(Z+1)`; do not yank in place of a fix.
## Copy-paste prompt

```md
Release `isomoes/blivedm_rs` as `vX.Y.Z` (date `YYYY-MM-DD`).

1. Start from a clean, up-to-date `master`; review commits since the last tag and choose
   the SemVer bump.
2. Add `## [X.Y.Z] - YYYY-MM-DD` to `CHANGELOG.md` under an empty `## [Unreleased]`.
3. Bump `Cargo.toml`, refresh `Cargo.lock`, then run `cargo test --locked` and
   `cargo build --locked`.
4. Commit only Cargo.toml, Cargo.lock, and CHANGELOG.md as `release vX.Y.Z`.
5. Create annotated tag `vX.Y.Z` on that commit and push `master`, then the tag.
6. Verify the workflow, crates.io version, and GitHub Release; report all URLs.

Do not force-push, move a pushed tag, or republish a version crates.io accepted.
```
