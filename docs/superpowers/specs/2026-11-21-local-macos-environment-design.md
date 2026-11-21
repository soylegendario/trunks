# Local macOS development environment — design

## Intent

Prepare this Mac to work on the repository's existing Tauri 2, Svelte 5/TypeScript, and Rust stack.

## Scope and constraints

- Reuse the installed Node.js 22, npm, and Xcode toolchain.
- Install stable Rust with rustup and install frontend dependencies from `package-lock.json` using `npm ci`.
- Verify the frontend with `npm run check` and `npm run build`, and the Rust backend with `cargo check --manifest-path src-tauri/Cargo.toml`.
- Limit setup to this macOS host; do not change application code, dependency declarations, or lockfiles.

## Approaches

- **Native host setup (selected):** install the missing Rust toolchain and repository dependencies on macOS. This directly supports Tauri's native build path.
- **Frontend-only setup:** defer Rust and verify only Svelte/TypeScript. This leaves the native application incomplete for development.
- **Container setup:** not selected because it does not replace the host-native Tauri toolchain and platform integration needed here.

## Implementation calls

Use rustup's stable toolchain, add its default Cargo bin directory to this shell's PATH if needed, then install locked npm dependencies. Record any platform prerequisites or build failures rather than modifying project configuration to work around them.
