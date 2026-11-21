# Application Icon Design

**Status:** Approved by product owner

## Intent

Use the supplied `trunks_icon.png` as the Trunks desktop application icon across supported platforms.

## Approach

Copy the supplied source into `src-tauri/icons/app-icon.png`, then generate the platform icon set with the existing Tauri CLI into `src-tauri/icons`. Keeping the source in the repository makes regeneration reproducible. The current `tauri.conf.json` already references the standard PNG, ICNS, and ICO outputs, so no configuration changes are needed. Manual per-size conversion risks inconsistent assets; replacing only one PNG would leave native platform icons unchanged.

## Scope

- Add the supplied PNG as `src-tauri/icons/app-icon.png` and replace generated icon assets in `src-tauri/icons` from it.
- Preserve existing icon paths and bundle configuration.
- Do not redesign or edit the source artwork.

## Verification

Inspect the generated asset list and run the frontend type check and production build. A full native bundle is outside this small asset change.
