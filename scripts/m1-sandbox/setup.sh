#!/usr/bin/env bash
# Builds a disposable Git sandbox for the manual M1 pass (AC-M1-01 .. AC-M1-08).
# See docs/milestones/m1-manual-pass.md for the checklist that uses it.
#
# Usage: scripts/m1-sandbox/setup.sh [sandbox-dir]
#   Default sandbox-dir: ${TMPDIR:-/tmp}/trunks-m1-sandbox
#
# Idempotent: re-running wipes and rebuilds ONLY a directory this script created
# (it must contain the marker file). It refuses to build inside an existing Git
# repository, so it can never create nested repos in the Trunks checkout.
set -euo pipefail

ROOT="${1:-${TMPDIR:-/tmp}/trunks-m1-sandbox}"
ROOT="${ROOT%/}"
MARK=".trunks-m1-sandbox"

if [ -z "$ROOT" ] || [ "$ROOT" = "/" ]; then
  echo "Refusing to use '$ROOT' as sandbox directory." >&2
  exit 1
fi

if [ -e "$ROOT" ]; then
  if [ ! -f "$ROOT/$MARK" ]; then
    echo "Refusing to delete '$ROOT': it was not created by this script (no $MARK marker)." >&2
    exit 1
  fi
  rm -rf "$ROOT"
fi

PARENT="$(dirname "$ROOT")"
if git -C "$PARENT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Refusing to build the sandbox inside a Git repository ($PARENT). Choose another directory." >&2
  exit 1
fi

mkdir -p "$ROOT/worktrees"
touch "$ROOT/$MARK"

g() {
  git -c user.name="Sandbox" -c user.email="sandbox@trunks.local" \
      -c commit.gpgsign=false -c init.defaultBranch=main "$@"
}

REPO="$ROOT/repo ñ con espacio"
ORIGIN="$ROOT/origin.git"
WT="$ROOT/worktrees"

# --- Remote, so remote-tracking branches exist -------------------------------
g init -q --bare -b main "$ORIGIN"

# --- Main repository (path with spaces and Unicode on purpose) ---------------
mkdir -p "$REPO/src/nested/deep"
g -C "$REPO" init -q -b main
printf '# Sandbox\nline 2\nline 3\n' > "$REPO/README.md"
printf 'app v1\n' > "$REPO/src/app.txt"
printf 'contenido con ñ\n' > "$REPO/src/nested/ñandú.txt"
printf 'deep\n' > "$REPO/src/nested/deep/leaf.txt"
printf 'stable content\nline 2\nline 3\n' > "$REPO/old-name.txt"
g -C "$REPO" add -A
g -C "$REPO" commit -q -m "initial"
printf 'app v2\n' > "$REPO/src/app.txt"
printf '\x89PNG\x00\x01\x02binary' > "$REPO/logo.bin"
g -C "$REPO" add -A
g -C "$REPO" commit -q -m "second commit"
g -C "$REPO" remote add origin "$ORIGIN"

# --- Branches ----------------------------------------------------------------
for b in feature/login "feature/ñandú" bugfix/typo release/1.0 prunable/gone; do
  g -C "$REPO" branch "$b"
done
g -C "$REPO" push -q origin main feature/login release/1.0
# Remote-only branch: pushed, then the local branch is deleted.
g -C "$REPO" branch remote-only
g -C "$REPO" push -q origin remote-only
g -C "$REPO" branch -D remote-only >/dev/null

# --- Worktrees (AC-M1-02) ----------------------------------------------------
g -C "$REPO" worktree add -q "$WT/wt-feature-login" feature/login
g -C "$REPO" worktree add -q "$WT/wt con espacio ñ" "feature/ñandú"
g -C "$REPO" worktree add -q --detach "$WT/wt-detached" HEAD~1
g -C "$REPO" worktree add -q "$WT/wt-locked" bugfix/typo
g -C "$REPO" worktree lock --reason "sandbox: locked on purpose" "$WT/wt-locked"
g -C "$REPO" worktree add -q "$WT/wt-prunable" prunable/gone
rm -rf "$WT/wt-prunable"   # directory vanishes -> Git reports it as prunable
g -C "$REPO" worktree add -q -b wip/clean "$WT/wt-clean"

# --- Dirty worktree (AC-M1-06 guard, and AC-M1-07/08 for Task 4) -------------
g -C "$REPO" worktree add -q -b wip/dirty "$WT/wt-dirty"
D="$WT/wt-dirty"
printf '# Sandbox\nline 2 changed\nline 3\n' > "$D/README.md"                 # unstaged
printf 'app v3 staged\n' > "$D/src/app.txt"; g -C "$D" add src/app.txt        # staged
g -C "$D" mv old-name.txt "renamed ñ.txt"                                      # staged rename
printf 'uno\ndos\ntres' > "$D/untracked text.txt"                              # untracked, no trailing newline
printf '\x00\x01\x02binary-untracked' > "$D/untracked.bin"                     # untracked binary
head -c 1100000 /dev/zero | tr '\0' 'a' > "$D/untracked-big.txt"               # untracked > 1 MiB
mkdir -p "$D/new-dir"; printf 'x\n' > "$D/new-dir/a.txt"                       # untracked directory

# --- Not a repository (AC-M1-01 negative case) -------------------------------
mkdir -p "$ROOT/not-a-repo"
printf 'just a folder\n' > "$ROOT/not-a-repo/file.txt"

# --- External changes while the app is open (AC-M1-05, AC-M1-07) -------------
cat > "$ROOT/external.sh" <<'EOS'
#!/usr/bin/env bash
# Usage: external.sh add|remove|touch   (run it while Trunks is open, then refresh)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$HERE/repo ñ con espacio"
case "${1:-}" in
  add)    git -C "$REPO" worktree add -b external/added "$HERE/worktrees/wt-external" ;;
  remove) git -C "$REPO" worktree remove --force "$HERE/worktrees/wt-external"
          git -C "$REPO" branch -D external/added ;;
  touch)  echo "change $(date +%s)" >> "$HERE/worktrees/wt-clean/README.md" ;;
  *)      echo "usage: $0 add|remove|touch" >&2; exit 2 ;;
esac
EOS
chmod +x "$ROOT/external.sh"

echo "Sandbox ready: $ROOT"
echo
echo "== git worktree list (main repo) =="
g -C "$REPO" worktree list
echo
echo "== branches =="
g -C "$REPO" branch -a
echo
echo "== status of wt-dirty =="
g -C "$D" status --short
