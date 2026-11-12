# Language

- Use English for repository documentation, comments, commit messages, and agent communication.
- Preserve literal user content, bilingual examples, identifiers, and quoted text where translation would change their meaning.

# Documentation ownership

- The only independently maintained project documents are the top-level `README.md`, `YPST.md`, and `GLOSSARY.md`.
- `README.md` is the concise user manual: installation, essential usage, supported behavior, and user-visible limitations.
- `YPST.md` is the developer manual: architecture, implementation details, build/test/release workflows, asset maintenance, and design decisions.
- `GLOSSARY.md` defines agreed terminology, keywords, and naming conventions. Link to it instead of redefining terms inconsistently.
- This `AGENTS.md` is the workflow instruction file, not a fourth project manual. Keep contributor rules here and technical explanations in `YPST.md`.
- Do not create or independently edit additional guides, nested READMEs, changelogs, or task reports. Update the appropriate top-level document instead.
- Release archives may copy these canonical documents; do not maintain separate archive-only prose. Third-party licenses and notices remain with their assets.
- Keep browser styles, scripts, and HTML-specific assets under `html/`; do not introduce parallel root-level or `web/` copies.
- When changing documented behavior or moving files, update the owning document, links, embedded resources, and packaging references together.

# Package version policy

- Keep this package permanently at version `0.1.0`.
- Do not create, publish, install, or migrate to another package version.
- Keep `typst.toml` set to `version = "0.1.0"`.
- When synchronizing the local Typst package, update only
  `%APPDATA%/typst/packages/local/ypst-template/0.1.0/`.

# Validation

- Prefer low-cost, targeted checks; run integration tests only when necessary and with the smallest relevant scope.
- For browser checks, reuse global Playwright at `/home/jay-waves/.local/lib/node_modules/playwright` and system Chrome with `chromium.launch({ channel: "chrome" })`.
- Do not add Playwright to projects, download browsers, or reinstall Playwright for ad hoc checks without user approval.

# Change and delivery workflow

- After Rust changes, run `cargo fmt`, then `cargo fmt --check` and relevant tests before committing.
- Before committing, run `git diff --check` and review the staged diff; exclude secrets and unrelated files.
- When asked to push, use SSH key authentication and report the commit and verification results.
