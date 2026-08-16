# Changelog

All notable changes to this project will be documented in this file.



## [Unreleased]
- Add `--json` and `--md` export flags to emit reports
- Add `src/export.rs` for JSON/Markdown serialization and file output
- Add unit tests for export serialization
- Refine `TODOs` checker to only match TODO/FIXME/XXX/HACK in common comment syntaxes
- Exclude generated reports (`ship-report.md`, `ship-report.json`) and common build dirs (`target/`, `.git/`) from scans
- Add `CHANGELOG.md`

## [0.1.0] - 2026-08-16
- Initial public release
