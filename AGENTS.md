# Package version policy

- Keep this package permanently at version `0.1.0`.
- Do not create, publish, install, or migrate to another package version.
- Keep `typst.toml` set to `version = "0.1.0"`.
- When synchronizing the local Typst package, update only
  `%APPDATA%/typst/packages/local/ypst-template/0.1.0/`.

# Change and delivery workflow

- After Rust changes, run `cargo fmt`, then `cargo fmt --check` and relevant tests before committing.
- Before committing, run `git diff --check` and review the staged diff; exclude secrets and unrelated files.
- When asked to push, use SSH key authentication and report the commit and verification results.
