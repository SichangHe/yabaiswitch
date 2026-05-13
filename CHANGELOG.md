# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/SichangHe/yabaiswitch/compare/v0.1.1...v0.1.2) - 2026-05-13

### Added

- add --exclude-apps flag, space command, and optimize subprocess calls

### Fixed

- focus_space runs 10-20 iters, exits only after target confirmed twice
- apply focus_space retry in both branches of space_focus
- focus_space retry for empty spaces; allow already-focused window
- retry window focus every 10ms if macOS steals it back (up to 10x)

### Other

- gate debug notifications behind cfg(debug_assertions)
- EXCLUDE_APPS env var, runtime debug flag, dedup notify
- debug code for exclude failure
- bump sccache-action v0.0.5 -> v0.0.9
- tighten release profile for smaller binary and faster startup
- yabai manual for agent

## [0.1.1](https://github.com/SichangHe/yabaiswitch/compare/v0.1.0...v0.1.1) - 2024-12-18

### Other

- change hardcoded yabai string;ci update
