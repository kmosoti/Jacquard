# Current Project State

## Active Work

**Milestone 02, verification foundation**, on `milestone/02-verification-foundation`: `jacquard-xtask` with `check-layers`, `check-core-purity`, `check-trust-boundary`, the check registry and receipts, fixtures for every failure code, a calibration record, and CI. Next: create the remote and push, then milestone 03.

## Milestones

| Nn | Name | Lands | Status |
| --- | --- | --- | --- |
| 01 | architecture-foundation | workspace, layers, contract, glossary, ADR-0000..0003, research digest | landed |
| 02 | verification-foundation | `jacquard-xtask`: `check-layers`, `check-core-purity`, `check-trust-boundary`, check registry with receipts, fixture negative controls, CI | landed |
| 03 | factory-ports | Loom, Renderer, Checker, Judge and Ledger ports and their first adapters; Judge admission set | planned |
| 04 | semantic-kernels | Bradley–Terry ranking with reference implementation and metamorphic relations; semantic-mutant corpus | planned |
| 05 | product-installation | the Installation imported as the first Product from a clean snapshot; its own contract and Checks | planned |
| 06 | local-filter | llama.cpp with a small VLM answering fixed structured questions, measured against fixtures | planned |

The remote repository is created after milestone 02, so the first push already runs the gates. The semantic-mutant corpus and a scheduled `cargo mutants` run belong to milestone 04, with the kernels they grade.

## Implemented

- Workspace with six crates across five layers ([hexagon](architecture/hexagon.md), [layers.json](architecture/layers.json)).
- `jacquard-core`: `Outcome`, `Verdict`, `Judgment`, `Vote`, `DraftId`, `SwatchId`, and the module contracts.
- `jacquard-xtask`: four gates, the check registry (`verification/checks.toml`), receipts, and `checks --profile fast`, which CI runs on every push. 63 fixture cases; every failure code has one. A hand-injected-defect calibration caught 7 of 9 defects; the 2 survivors are classified in `verification/calibration/2026-09-28-gates.md`.
- Product contract, glossary, ADR-0000 to ADR-0003, the [design-loops research digest](research/2026-09-28-design-loops/README.md).

## Known Gaps

- `check-trust-boundary` does not inspect conflict resolutions inside merge commits.
- Receipts are not signed; a receipt proves structure and that the recorder ran the command.
- The `std` scan in `check-core-purity` is syntactic; effects reached through an allowlisted crate are a review matter (the allowlist is empty).

## Current Assumptions

- Sonnet 5.5 is both Loom and sole Judge ([ADR-0002](decisions/ADR-0002-sonnet-sole-judge.md)); self-preference is mitigated, not removed.
- Local models run on one machine: RTX 3060 Laptop 6 GB, 15 GB RAM, WSL2. They filter; they never rank.
- Designers agree with each other at Krippendorff α≈0.25 on UI preference, so the owner's Votes, not a population, are the calibration target.

## Unresolved Questions

- License (Nomos uses Apache-2.0; not yet chosen here).
- Whether `jacquard-judge-local` is worth its adapter before milestone 05 shows what the Checks miss.
- The Installation's own contract clauses (its "machine is the narrative" rule) belong to milestone 05.
