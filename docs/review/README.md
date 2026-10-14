# Maintenance and feasibility reports

Reviewed on 2026-10-14, in the requested order. Reports contain grouped causes, source evidence, implementation proposals, overlaps and validation boundaries.

**Current status:** [Selected maintenance implementation](06-maintenance-implementation.md) applies C1 and A01–A08/A12–A15/A17–A20 with Rust 1.99.0. Strict Clippy passes; A09/A10/A11/A16/A21 and the exploratory features remain deferred. The original report outcomes below describe the initial review, not today's final code.

| Task | Report | Outcome |
| --- | --- | --- |
| Maintenance 1: Clippy | [01-clippy.md](01-clippy.md) | 114 warnings, 57 distinct locations, one shared error-size issue; deferred. Two harmless rustfmt changes applied in `clone.rs`. |
| Maintenance 2: overall app | [02-application-review.md](02-application-review.md) | 21 grouped findings; report-only. A20 explicitly overlaps Clippy C1. |
| Exploration 1: config/secrets | [03-configuration.md](03-configuration.md) | Feasible: TOML profiles, defined precedence and secret providers. |
| Exploration 2: PostgreSQL tools/install | [04-postgres-tools.md](04-postgres-tools.md) | Feasible: improve resolver/install first, then portable suites and a verified managed cache. |
| Exploration 3: Windows | [05-windows.md](05-windows.md) | Feasible port; Unix permissions/temp paths, passfiles, native dependencies and release tests need work. |

First maintenance priorities are credential logging/passfile handling, TLS verification, restore error propagation, SQL construction and destructive-operation preflight. The exploratory designs share the credential resolver, private file handling and tool-suite resolver; implement those once rather than once per feature.

Verification: all 7 existing unit tests passed; debug build and final formatting checks passed. Strict Clippy still fails on C1. Four disposable local probes confirmed utility-command credential dependency, wildcard-passfile panic and acceptance of failing version commands. No PostgreSQL/S3 operations, Windows build, deployment, dependency update, commit or push was performed. Proposed command syntax and config examples are future designs, not implemented features.
