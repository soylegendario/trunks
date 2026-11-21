# CLAUDE.md

> **IMPORTANTE PARA CLAUDE:** Este repositorio utiliza un protocolo de colaboración multi-agente centralizado.
> **DEBES LEER Y SEGUIR RIGUROSAMENTE [`AGENTS.md`](AGENTS.md)** antes de realizar cualquier cambio o tomar decisiones sobre el proyecto.

---

## Resumen Rápido de Inicio

1. **Lee `AGENTS.md`**: Contiene las restricciones arquitectónicas innegociables (no convertir en IDE, no orquestar IA en la app, Git como única fuente de verdad) y el protocolo de handoff.
2. **Revisa el estado actual**:
   - `README.md` (`## Current stage`)
   - Plan activo en `docs/superpowers/plans/` (actualmente `docs/superpowers/plans/2026-11-20-m0-terminal-workspace.md`)
3. **Identifica la siguiente tarea**: Busca la primera tarea pendiente (`- [ ]`) en el plan activo.
4. **Comandos de desarrollo**:
   - Frontend: `npm run check && npm run build`
   - Rust: `cargo check --manifest-path src-tauri/Cargo.toml` / `cargo test --manifest-path src-tauri/Cargo.toml`
   - Dev: `npm run tauri dev`
   *(Si `cargo` no está en PATH, ejecutar `export PATH="$HOME/.cargo/bin:$PATH"`)*.
