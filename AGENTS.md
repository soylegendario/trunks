# AGENTS.md — Protocolo de Colaboración Multi-Agente

Este repositorio (**Trunks / Terminal Workspace**) se desarrolla de forma colaborativa y asíncrona mediante múltiples agentes de Inteligencia Artificial (Junie, Codex, Claude, etc.) y desarrolladores humanos.

El objetivo de este documento es maximizar la eficiencia colectiva, evitar la duplicación de trabajo, preservar el contexto entre sesiones y garantizar una transición fluida e inmediata para cualquier agente que tome el control del proyecto.

---

## 1. Protocolo de Inicio de Sesión (Lectura Obligatoria)

Al comenzar una nueva sesión, **todo agente DEBE realizar los siguientes pasos en orden antes de ejecutar cualquier cambio**:

1. **Leer este archivo (`AGENTS.md`)** para recordar las reglas de juego y restricciones.
2. **Consultar el estado actual del proyecto**:
   - Revisar la sección `## Current stage` en [`README.md`](README.md).
   - Revisar el plan activo en [`docs/superpowers/plans/`](docs/superpowers/plans/) (actualmente [`docs/superpowers/plans/2026-11-21-m1-git-worktrees.md`](docs/superpowers/plans/2026-11-21-m1-git-worktrees.md)).
3. **Identificar la tarea inmediata**:
   - Localizar el primer elemento pendiente (`- [ ]`) en el plan activo.
   - Contrastar los requisitos con [`docs/spec.md`](docs/spec.md), [`docs/milestones/m1-git-worktrees.md`](docs/milestones/m1-git-worktrees.md) y el ADR correspondiente en [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md).

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

- **No convertir el proyecto en un IDE**: No añadir LSP, depuradores ni edición de archivos dentro del alcance inicial. Trunks es un espacio de trabajo para terminales y navegación/inspección ligera.
- **Sin integración/orquestación de IA propietaria**: Trunks corre en local y no gestiona prompts, APIs de proveedores ni orquesta sesiones de agentes de forma interna. Los agentes CLI se ejecutan como cualquier otro comando de terminal dentro del PTY.
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
- **Hito:** Milestone 1 (M1 — Worktree-First Project Workflow & Git Integration).
- **Rama de trabajo:** el código de las Tareas 1 y 2 vive en la rama `feat/m1-git-backend` y **no está en `main` hasta que se fusione**. Quien continúe con la Tarea 3 debe partir de esa rama (o de `main` si ya se fusionó). Al fusionarla, eliminar esta línea.
- **Progreso:**
  - [x] **Milestone 0 Completado:** Scaffold Tauri 2 + Svelte 5, PTY nativo y fidelidad de terminal (`portable-pty` + xterm.js), persistencia SQLite transaccional (WAL mode) con reconciliación de rutas y recuperación en frío sin procesos fantasma (`docs/milestones/m0-results.md`, `docs/adr/0001-stack.md`).
  - [x] **M1 - Tarea 1 Completada:** Servicio nativo de Git en Rust (`src-tauri/src/git.rs`; 8 comandos Tauri registrados; 43 tests, incluidos de integración contra `git` real, que exigen `git` en el PATH). Notas y desviaciones en el plan activo.
  - [x] **M1 - Tarea 2 Completada:** Integración de UI para registro de repositorios Git y gestión de worktrees en barra lateral izquierda (`src/lib/git.ts`, `src/lib/projects.svelte.ts`, componentes modales y `WorktreeList.svelte`). Verificación manual **parcial** y deuda conocida: ver la sección "Task 2 review and manual pass" del plan activo.
  - ✅ **Decisión tomada (caché de worktrees):** SQLite guarda solo la referencia del proyecto (id, nombre, ruta) y la selección; los worktrees se leen siempre de Git, sin caché. La lógica está en `src/lib/project-logic.ts` y se prueba con `npm run test:unit`. La Tarea 3 debe enlazar pestañas con la ruta del worktree que informa Git.
  - [x] **M1 - Tarea 3 Completada (código):** pestañas enlazadas a la ruta del worktree que informa Git (`findTabForWorktree` en `project-logic.ts`; abrir terminal enfoca la pestaña existente; botón deshabilitado en worktrees prunable; `pty_create` falla si el cwd no existe). Pendiente la pasada manual (`pwd` y reinicio en frío), ver el plan.
  - [x] **M1 - Tarea 4 Completada (código):** panel inspector derecho (`InspectorPanel`, `GitStatusView`, `DiffViewer`, `FileTree`) con sondeo cada 2,5 s. Pendiente la pasada manual: `git_get_status`, `git_get_diff` y `git_list_files` aún no se han probado por IPC con la app (ver el plan).
  - [ ] **M1 - Tarea 5 (Siguiente):** Validación E2E de criterios `AC-M1-01` a `AC-M1-08` y consolidación de resultados en `docs/milestones/m1-results.md`.

---

## 5. Comandos de Desarrollo y Verificación

> **Nota para subshells:** Si el binario `cargo` no se encuentra en el PATH, ejecutar previamente `export PATH="$HOME/.cargo/bin:$PATH"`.

| Acción | Comando |
|---|---|
| Instalar dependencias frontend | `npm install` |
| Verificación de tipos y linting | `npm run check` |
| Compilación del frontend | `npm run build` |
| Tests unitarios de la lógica del frontend | `npm run test:unit` (runner integrado de Node, sin dependencias; ejecuta TypeScript de forma nativa, probado con Node 22.23: en versiones anteriores puede fallar) |
| Verificación rápida de Rust | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Compilación de Rust en debug | `cargo build --manifest-path src-tauri/Cargo.toml` |
| Ejecución en modo desarrollo | `npm run tauri dev` |
| Repo desechable para la pasada manual de M1 | `scripts/m1-sandbox/setup.sh` (guía: `docs/milestones/m1-manual-pass.md`) |

---

## 6. Convención de Mensajes de Commit (si aplica)
Cuando un agente realice commits (solo cuando el usuario lo solicite expresamente), debe incluir la autoría o coautoría correspondiente:
- Prefijos convencionales: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
