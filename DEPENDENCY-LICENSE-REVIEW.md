# Dependency License Review

This is a preliminary manifest and lockfile review of `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`. It is not a legal opinion or a substitute for checking notices in each release bundle.

## Compatible licenses observed

- The direct JavaScript runtime dependencies (`@tauri-apps/api`, `@tauri-apps/plugin-opener`, `@xterm/xterm`, and `@xterm/addon-fit`) declare MIT or MIT/Apache-2.0 alternatives.
- The direct JavaScript development tools (`@sveltejs/adapter-static`, `@sveltejs/kit`, `@sveltejs/vite-plugin-svelte`, `@tauri-apps/cli`, `svelte`, `svelte-check`, `typescript`, and `vite`) declare MIT or Apache-2.0 terms.
- The direct Rust dependencies (`tauri`, `tauri-plugin-opener`, `portable-pty`, `rusqlite`, `serde`, `serde_json`, and `tokio`) use permissive MIT and/or Apache-2.0 terms. Apache-2.0 is compatible with GPL v3.
- The reviewed transitive dependency metadata primarily declares MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, or Unlicense terms, which do not present an apparent GPL v3 compatibility conflict.

## Planned addition (M3)

- `@xterm/addon-serialize` 0.13.0 declares the MIT license and a peer dependency of `@xterm/xterm ^5.0.0`, which matches the installed 5.5. Later versions target xterm 6 and must not be used until xterm is upgraded. It will be added to the direct JavaScript runtime dependencies when Task 2 of the M3 plan installs it; no other dependency is added by M3. Metadata-only screening, same caveats as below.

## Requires verification

- MPL-2.0 crates appear in the Rust lockfile, including `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`. MPL 2.0 section 3.3 provides a route for distributing a larger work under GPL v3, subject to its conditions; confirm the relevant versions do not exclude secondary licenses and preserve the MPL notices and availability of MPL-covered files as required.
- `lightningcss` and its platform packages declare MPL-2.0 in the JavaScript lockfile as development/build-tool dependencies. They are not direct runtime dependencies and are not normally bundled as application code, but confirm this against the actual release build contents.
- Cargo has target-specific dependencies for native webview platforms. Review the dependency notices and actual linked/bundled components for each release target, rather than assuming every lockfile entry is present in every binary.

## Potential incompatibilities

No dependency clearly identified in this manifest/lockfile review appears incompatible with GPL v3. No dependencies were changed. This is a metadata-based screening; it does not verify every release artifact, optional feature, or upstream license exception.

## References

- [GNU GPL license compatibility guidance](https://www.gnu.org/licenses/license-list.html)
- [Mozilla MPL 2.0 FAQ, including GPL combination guidance](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)
