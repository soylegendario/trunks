# AGENTS.md — Protocolo de Colaboración Multi-Agente

Este repositorio (**Senda / Terminal Workspace**) se desarrolla de forma colaborativa y asíncrona mediante múltiples agentes de Inteligencia Artificial (Junie, Codex, Claude, etc.) y desarrolladores humanos.

El objetivo de este documento es maximizar la eficiencia colectiva, evitar la duplicación de trabajo, preservar el contexto entre sesiones y garantizar una transición fluida e inmediata para cualquier agente que tome el control del proyecto.

---

## 1. Protocolo de Inicio de Sesión (Lectura Obligatoria)

Al comenzar una nueva sesión, **todo agente DEBE realizar los siguientes pasos en orden antes de ejecutar cualquier cambio**:

1. **Leer este archivo (`AGENTS.md`)** para recordar las reglas de juego y restricciones.
2. **Consultar el estado actual del proyecto**:
   - Revisar la sección `## Current stage` en [`README.md`](README.md).
   - Revisar el plan activo en [`docs/superpowers/plans/`](docs/superpowers/plans/) (actualmente [`docs/superpowers/plans/2026-11-20-m0-terminal-workspace.md`](docs/superpowers/plans/2026-11-20-m0-terminal-workspace.md)).
3. **Identificar la tarea inmediata**:
   - Localizar el primer elemento pendiente (`- [ ]`) en el plan activo.
   - Contrastar los requisitos con [`docs/spec.md`](docs/spec.md), [`docs/milestones/m0-terminal-workspace-spike.md`](docs/milestones/m0-terminal-workspace-spike.md) y el ADR correspondiente en [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md).

---

## 2. Protocolo de Fin de Tarea / Handoff (Actualización Documental)

Para que el siguiente agente pueda continuar sin pérdida de contexto ni ambigüedad, **es obligatorio registrar documentalmente todo avance antes de terminar la sesión**:

1. **Actualizar el Plan de Trabajo**:
   - Marcar como completadas (`- [x]`) las tareas y subtareas finalizadas en `docs/superpowers/plans/<plan-activo>.md`.
   - Si una tarea queda a medias o bloqueada, añadir una nota concisa debajo de la subtarea con el motivo y el siguiente paso sugerido.
2. **Actualizar el `README.md`**:
   - Mantener sincronizada la sección `## Current stage` con el hito o tarea recién completada y el siguiente objetivo.
3. **Registrar Evidencia Técnica y Decisiones**:
   - Si se completaron criterios de aceptación de un milestone, documentar los resultados en `docs/milestones/<milestone>-results.md` (ej. `docs/milestones/m0-results.md`).
   - Si se propone o descarta un componente del stack, actualizar `docs/adr/0001-stack.md`.
4. **Verificación y Estado Limpio**:
   - Comprobar que el código compila sin errores antes de finalizar:
     - Frontend: `npm run check && npm run build`
     - Backend nativo: `cargo check --manifest-path src-tauri/Cargo.toml` (o `cargo build`)
   - Asegurarse de que no quedan archivos temporales o procesos colgados.

---

## 3. Principios y Restricciones Arquitectónicas Innegociables

Todo agente debe respetar los principios definidos en [`docs/intent.md`](docs/intent.md) y [`docs/spec.md`](docs/spec.md):

- **No convertir el proyecto en un IDE**: No añadir LSP, depuradores ni edición de archivos dentro del alcance inicial. Senda es un espacio de trabajo para terminales y navegación/inspección ligera.
- **Sin integración/orquestación de IA propietaria**: Senda corre en local y no gestiona prompts, APIs de proveedores ni orquesta sesiones de agentes de forma interna. Los agentes CLI se ejecutan como cualquier otro comando de terminal dentro del PTY.
- **Git como única fuente de verdad**: No crear bases de datos secundarias para el estado de Git. La aplicación consulta y reconcilia con el CLI de Git del sistema.
- **Identidad de Pestaña vs. Proceso PTY**:
  - *Terminal tab*: Identidad duradera en la UI vinculada a proyecto/worktree/directorio recordado.
  - *PTY process*: Instancia viva del proceso shell/CLI asignado a esa pestaña.
  - Al reiniciar la aplicación, las pestañas se restauran marcadas como "detenidas" (sin procesos fantasma).
- **Multiplataforma Nativo**:
  - macOS, Windows nativo (sin requerir WSL) y Linux.
  - Evitar dependencias que solo funcionen en Unix o que requieran herramientas externas obligatorias como `tmux` o `zellij`.

---

## 4. Estado Actual del Proyecto y Hoja de Ruta

### Estado actual
- **Hito:** Milestone 0 (M0 Spike - Terminal and workspace recovery).
- **Progreso:**
  - [x] **Tarea 1 completada:** Entorno aprovisionado (Node.js, Rust/Cargo). Scaffold base creado con Tauri 2, Svelte 5, TypeScript y Rust. Layout de 3 regiones implementado y dimensiones reactivas verificadas.
  - [x] **Tarea 2 completada:** Validado el ciclo de vida de PTY y fidelidad de terminal (`portable-pty` en Rust + `@xterm/xterm` en Svelte). Soporte multi-pestaña, I/O en tiempo real, redimensionado reactivo sin reinicio de proceso y tests automáticos superados en macOS (`docs/milestones/m0-results.md`).
  - [x] **Tarea 3 completada:** Persistencia incremental y recuperación de estado (`SQLite` / `rusqlite` en WAL mode). Reconciliación de rutas huérfanas/movidas, retención de orden de pestañas y recuperación en frío sin procesos fantasma (`AC-M0-06`, `AC-M0-07`, `AC-M0-08`).
  - [ ] **Tarea 4 (Siguiente):** Evaluación de criterios de aceptación (`AC-M0-01` a `AC-M0-09`) y consolidación de resultados en `docs/milestones/m0-results.md` y `docs/adr/0001-stack.md`.

---

## 5. Comandos de Desarrollo y Verificación

> **Nota para subshells:** Si el binario `cargo` no se encuentra en el PATH, ejecutar previamente `export PATH="$HOME/.cargo/bin:$PATH"`.

| Acción | Comando |
|---|---|
| Instalar dependencias frontend | `npm install` |
| Verificación de tipos y linting | `npm run check` |
| Compilación del frontend | `npm run build` |
| Verificación rápida de Rust | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Compilación de Rust en debug | `cargo build --manifest-path src-tauri/Cargo.toml` |
| Ejecución en modo desarrollo | `npm run tauri dev` |

---

## 6. Convención de Mensajes de Commit (si aplica)
Cuando un agente realice commits (solo cuando el usuario lo solicite expresamente), debe incluir la autoría o coautoría correspondiente:
- Prefijos convencionales: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
