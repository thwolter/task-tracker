# Tempo — Agent Guide

Tempo is a local-first desktop task and focus timer. Rust owns domain and
persistence logic; Slint owns UI logic where practical. Use stable Rust, SQLite,
and no cloud services or new dependencies without a clear reason.

## Changes

- Make the smallest maintainable change that fulfils the request.
- Preserve existing behaviour and unrelated worktree changes.
- Do not refactor unrelated code.
- Reuse existing Slint `Theme`, `Typography`, and shared components.
- Keep the UI usable at the configured minimum window size.

## Verification

- Run relevant tests and `cargo clippy` where practical.
- Do not run broad formatting for a narrow edit; format only intended files.
- Before completion, inspect `git status`, the full diff, and `git diff --check`.
- For UI changes, inspect the rendered state when possible; compilation alone is
  not visual acceptance.

## Completion

Report changed files, validation, limitations, and any pre-existing unrelated
worktree changes.
