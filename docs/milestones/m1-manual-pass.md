# M1 — Pasada manual (checklist)

Guía para validar a mano `AC-M1-01` .. `AC-M1-08` (los resultados definitivos se registran en `docs/milestones/m1-results.md`, Tarea 5).

## Preparación

1. `scripts/m1-sandbox/setup.sh` crea el repositorio desechable y **imprime su ruta** (por defecto `${TMPDIR:-/tmp}/trunks-m1-sandbox`). En esta guía `S` es esa ruta. Volver a ejecutarlo reinicia el escenario (cierra la app antes). Solo borra un directorio que el propio script creó y se niega a construirse dentro de otro repo Git.
2. Arranca la app desde la raíz del proyecto: `npm run tauri dev` (si `cargo` no está en el PATH: `export PATH="$HOME/.cargo/bin:$PATH"`).
3. Repo principal del escenario: `S/repo ñ con espacio`.

> **Los pasos destructivos (crear/borrar worktrees, 6–9 y 16–20) solo contra el sandbox.** Nunca los ejecutes sobre un repositorio real registrado en la app.

Anota por cada paso: OK / FALLA / observación. Los pasos **OBSERVAR** no tienen resultado esperado conocido.

## Escenario preparado

Worktrees (8): principal (`main`) · `wt-feature-login` (feature/login) · `wt con espacio ñ` (feature/ñandú) · `wt-detached` (detached) ·
`wt-locked` (bugfix/typo, locked) · `wt-prunable` (prunable/gone, directorio borrado) · `wt-clean` (wip/clean) · `wt-dirty` (wip/dirty, con cambios).
Ramas sin worktree: `release/1.0` (local y remota), `origin/remote-only` (solo remota); `feature/login` y `main` también están en `origin`.
`S/not-a-repo` es una carpeta normal. `S/external.sh add|remove|touch` aplica cambios externos.

## AC-M1-01 Registro

1. `+ Add Project` con `S/repo ñ con espacio` → aparece con nombre "repo ñ con espacio" y sus worktrees.
2. Añadir `S/not-a-repo` → error claro, NO se registra nada.
3. Añadir `S/repo ñ con espacio/src/nested/deep` (subcarpeta) → se normaliza a la raíz, NO duplica el proyecto.
4. Cerrar y reabrir la app → el proyecto sigue ahí.

## AC-M1-02 Descubrimiento (esperado: 8 entradas)

5. Cada worktree con su rama y ruta correctas; `wt-detached` sin rama; `wt-locked` con candado (motivo "sandbox: locked on purpose" en el tooltip); `wt-prunable` marcado `stale`; el principal con la etiqueta `primary`; las rutas con `ñ` y espacios se ven bien (sin `\303\261`).

## AC-M1-03 Creación

6. Rama nueva `test/new`, destino `S/worktrees/wt-new` → aparece; existe el directorio; `git -C "S/repo ñ con espacio" worktree list` lo muestra.
7. Rama existente `release/1.0`, destino `S/worktrees/wt-release` → aparece con esa rama.
8. Rama nueva `main` (ya existe) → error de git visible, no se crea nada.
9. Destino relativo (`rel/dir`) → error "must be absolute", no se crea nada dentro del repo.
10. OBSERVAR: crear un worktree en un proyecto que NO es el seleccionado (registra un 2.º repo **desechable**) → ¿qué proyecto/worktree queda seleccionado?
11. OBSERVAR: elegir `origin/remote-only` como rama existente.

## AC-M1-04 Terminal (parcial: lo completa la Tarea 3)

12. Botón `>_` en `wt-feature-login` → `pwd` = su ruta. Repetir en `wt con espacio ñ`.
13. OBSERVAR: pulsar `>_` dos veces en el mismo worktree (la Tarea 3 pide enfocar la pestaña existente).

## AC-M1-05 Sincronización externa

14. `S/external.sh add` → botón de refresco (y también cambiar de ventana y volver) → aparece `wt-external`.
15. `S/external.sh remove` → refrescar → desaparece. Reiniciar la app → sin corrupción ni entradas fantasma.

## AC-M1-06 Borrado guardado

16. Quitar `wt-clean` (🗑) → modal de confirmación → se borra el directorio.
17. Quitar `wt-dirty` SIN force → error de git visible; el directorio y `untracked text.txt` siguen en disco (`ls`).
18. Quitar `wt-dirty` CON force confirmado explícitamente → se borra.
19. El worktree principal no ofrece borrado (y el backend lo rechaza si se intenta).
20. OBSERVAR: quitar `wt-locked` (con y sin force; git exige doble `--force`) y `wt-prunable`. Anota el mensaje que ve el usuario. Pulsar `>_` en `wt-prunable` (el directorio no existe).
21. Quitar el PROYECTO (`✕`, no el worktree) → desaparece de Trunks y `S/repo ñ con espacio` sigue intacto en disco.

## Casos extra

22. Con 2 proyectos, cerrar y reabrir → cada proyecto muestra "Querying git worktrees..." hasta que git responde, y nunca aparecen worktrees de la sesión anterior antes de esa respuesta (ya no hay caché). El proyecto y el worktree que estaban seleccionados se restauran.
23. Stale: renombrar temporalmente `S/repo ñ con espacio` → refrescar → proyecto marcado stale con el error; devolver el nombre → refrescar → se limpia.
24. Foco de ventana repetido (5–10 veces seguidas) → sin parpadeos ni errores en la consola de devtools.
25. Consola de devtools abierta durante todo: apunta cualquier error o warning.
26. Barra lateral: con varios repos, scroll de la lista, colapsar/expandir cada proyecto y comprobar que el estado se recuerda al reabrir.
27. Con un worktree seleccionado, cerrar la app, quitarlo desde fuera (`git worktree remove`) y reabrir → al cargar se selecciona el primer worktree del proyecto, sin dejar una selección inválida.

## Preparado para la Tarea 4 (no es de esta pasada)

Comprobaciones añadidas tras los cambios de UI: Cmd+B y Cmd+L muestran/ocultan los paneles (también con el foco en una terminal); arrastrar el borde de cada panel lo redimensiona y el ancho se recuerda al reabrir; pulsar un archivo en Files o Git abre una pestaña de solo lectura en el centro; `new-dir/` lista sus archivos agrupados por carpeta plegable; la bolita verde/naranja aparece en el worktree con terminales en ejecución/restauradas.


Estado esperado de `wt-dirty`: ` M README.md` · `M  src/app.txt` · `R  old-name.txt -> renamed ñ.txt` · `?? new-dir/` ·
`?? untracked text.txt` (3 líneas, sin salto final) · `?? untracked.bin` (binario) · `?? untracked-big.txt` (>1 MiB → "too large").
`S/external.sh touch` modifica `wt-clean/README.md` para probar el refresco del inspector.
