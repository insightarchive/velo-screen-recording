# Velo Recorder — working agreement for Claude

Velo Recorder is a standalone Rust + Tauri screen recorder (see `README.md` for what it does and
the layout). It is being built as its own app for now, and will be merged into **VELO Power
Suite** (`D:\NEXUS-productivity-apps`, the owner's main productivity app) once it is ready. The
owner is Bangla-speaking and wants to stop worrying about the mechanics: **handle everything
yourself, end to end, the way a senior engineer running their own branch would.**

## How to work (read this before touching anything)

1. **Own the whole loop yourself — don't hand steps back to the owner.** When you finish a piece
   of work: build it, run it, actually click through the feature in the real app (not just
   `cargo check`), fix what's broken, and only then say it's done. A "should work" that was never
   run in the real app is not done.
2. **Commit and push on your own, after each finished piece — don't wait to be told.** Small,
   focused commits over one giant one. Write a real commit message: what changed and, where it
   isn't obvious from the diff, why — not "fix stuff" or "update". Plain fast-forward pushes to
   `origin main`; never `--force`. If something is genuinely destructive or outward-facing (force
   push, discarding uncommitted work, rewriting history), stop and ask first — everything else,
   just do it.
3. **Verify for real, not by compiling.** `cargo build` passing is not verification. Run the app,
   trigger the feature (recording start/stop/pause, the pill bar, hotkeys, Studio vs Quick mode),
   and look at the actual result — a real recorded file, the real UI state. Say plainly what you
   could not test (needs a second monitor, needs real audio hardware, etc.) instead of quietly
   skipping it.
4. **No half-finished work presented as done.** If a feature needs UI + backend + a settings
   toggle to be complete, finish all of it before calling it done — not a backend stub with a
   "TODO: wire up UI" left behind.
5. **Native Rust only.** No shelling out to PowerShell/AutoHotkey for things the Windows API or a
   crate can do directly — this project already hooks capture, audio and input natively
   (`src-tauri/src/engine/`), keep that standard.
6. **Never fake data or a fake recording.** Every number, every preview, every timeline segment
   shown in the UI comes from a real captured frame/event — nothing simulated presented as real,
   except the explicitly browser-only simulated-data mode `src/` already supports for pure design
   work (that one's fine, it's labelled and intentional).
7. **This becomes part of VELO.** Keep that merge in mind — avoid naming collisions with VELO's
   own conventions, keep the crate/module boundaries clean (`velo-core`, `velo-director`,
   `src-tauri/src/engine/`) so folding this into the main app later is a clean import, not a
   untangling job.

## Talking to the owner

- Reply in **Bangla script** (not Banglish) when explaining what's done, what's left, or why
  something broke — plain words, honest, a done-vs-left list when there's a list to give.
- He is often limited by usage quota — keep steps self-contained, don't start something huge
  unasked, and don't run a build/test he's explicitly declined.
- If he says "না / থামো / এখন করিও না" — stop immediately and say exactly what state things are
  left in (what's committed, what's still uncommitted, what's running).

## Git and testing

- `cargo test -p velo-director -p velo-core` before a push that touches either crate — keep tests
  meaningful (real inputs/outputs), not tautologies.
- Check `git status` before anything that could discard uncommitted work. If you find someone
  else's (or your own earlier, uncommitted) in-progress changes sitting in the tree, don't discard
  or silently fold them into an unrelated commit — understand what they are first.
