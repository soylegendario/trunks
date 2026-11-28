# Cross-platform GitHub Actions CI — Design (short)

Issue: proposed. Size: small change (GitHub Actions workflows only; no application code changes).

## Intent and why

Build Trunks on native Linux, macOS, and Windows runners so pull requests can catch platform-specific compilation and packaging failures and provide installers that reviewers can download for testing.

## Goals

- Run frontend checks and unit tests, plus Rust checks and tests, on pull requests.
- Build a native Tauri bundle for Linux, macOS, and Windows.
- Publish a separate downloadable workflow artifact for each operating system.
- Keep pull request artifacts temporary and publish versioned bundles to GitHub Releases when a version tag is pushed.

## Non-goals / anti-goals

- Do not sign or notarize macOS or Windows packages; release publication must not require signing secrets.
- Do not claim that a successful build proves runtime behavior on that operating system. Windows and Linux still need manual acceptance.
- Do not add a self-hosted runner, paid larger runner, or third-party CI service.

## Constraints

- GitHub Actions standard hosted runners are free for this public repository; use standard runners only.
- Use native runners: Ubuntu x64, macOS arm64, and Windows x64. Do not cross-compile desktop bundles between operating systems.
- Tauri's Linux system libraries must be installed before building.
- `src-tauri/tauri.conf.json` currently sets `bundle.targets` to `all`; retain that behavior.
- Pull request workflows must work without repository secrets, including pull requests from forks.
- Restrict workflow token permissions to read-only unless a job needs more.

## Approaches considered

- **A (chosen):** Keep pull request CI read-only and use a separate tag-triggered release workflow. Both build on the native three-OS matrix; PR builds upload workflow artifacts with 7-day retention, while version-tag builds publish bundles to a GitHub Release. This keeps write permissions out of PR workflows and provides both review builds and durable release downloads.
- **B (rejected):** Run checks on Linux only and produce no per-OS packages. Cheaper in runner time, but it misses the requested OS-specific bundles and platform build failures.
- **C (rejected):** Give one workflow write permissions for both PR CI and releases. It is simpler to maintain, but unnecessarily broadens the permissions of the workflow that handles pull requests.

## Design

- Add a CI workflow triggered by pull requests and pushes to `main`.
- Verification job on Ubuntu runs `npm ci`, `npm run check`, `npm run test:unit`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml`. The native Tauri build runs the frontend production build on each OS, so do not repeat `npm run build` as a separate CI step.
- CI build job uses a matrix of `ubuntu-24.04`, `macos-15`, and `windows-2022`, installs the platform prerequisites and project dependencies, then runs `npm run tauri build`.
- Upload files under `src-tauri/target/release/bundle/` as distinct artifacts named for the OS, retaining them for 7 days. Configure upload to fail if no bundle files are found rather than silently producing an empty artifact.
- Add a separate release workflow triggered by version tags matching `v*`. It builds all three native bundles and publishes them to a GitHub Release for that tag.
- Release assets are unsigned and macOS packages are not notarized. The release workflow requires no signing secrets and receives `contents: write`; pull request CI has read-only permissions.
- Unsigned macOS downloads may be blocked or flagged by Gatekeeper. Do not add an ad-hoc or Developer ID signature to work around this; document the limitation for testers.
- A tag must identify the same application version configured for the build; mismatches fail instead of publishing misleading asset names.
- No application files, dependency versions, or bundle targets change.

## Acceptance criteria

- A pull request starts verification and all three native build jobs.
- A successful run exposes one non-empty artifact per OS, containing the bundle(s) produced by Tauri for that runner.
- A failing verification or build job fails the workflow.
- Artifacts expire after 7 days.
- Pushing a matching `v*` tag creates or updates a GitHub Release containing unsigned bundles for Linux, macOS, and Windows.
- A tag/version mismatch fails the release workflow; pull request CI cannot publish releases.
- The workflows succeed without signing secrets. Only the release workflow has permission to write release assets.
- CI results are described as build validation only; they do not count as manual runtime acceptance for Windows or Linux.

## Left to the builder

- Choose stable, maintained versions of the official setup/upload actions and pin them according to the repository's security policy.
- Use the Tauri-documented Ubuntu package list and confirm the bundle output paths for each OS while implementing the workflow.
- Confirm the exact version-tag/release naming behavior and use a release action that uploads the generated bundles without enabling code signing.
- Add branch protection requiring the verification and platform build checks only after the workflow's check names are stable; branch protection is not part of this change.
