# Current Project State

## Active Work

**Milestone 01, architecture foundation**, landed as the initial commit on `main`: the hexagonal skeleton, the product contract, the vocabulary, the first decisions and the research digest. Crates carry module docs and the types that pin the evidence rule. No gate runs yet. Milestone 02 starts on `milestone/02-verification-foundation`.

## Milestones

| Nn | Name | Lands | Status |
| --- | --- | --- | --- |
| 01 | architecture-foundation | workspace, layers, contract, glossary, ADR-0000..0003, research digest | landed |
| 02 | verification-foundation | `jacquard-xtask`: `check-layers`, `check-core-purity`, `check-trust-boundary`, check registry with receipts, fixture negative controls, CI | next |
| 03 | factory-ports | Loom, Renderer, Checker, Judge and Ledger ports and their first adapters; Judge admission set | planned |
| 04 | semantic-kernels | Bradley–Terry ranking with reference implementation and metamorphic relations; semantic-mutant corpus | planned |
| 05 | product-installation | the Installation imported as the first Product from a clean snapshot; its own contract and Checks | planned |
| 06 | local-filter | llama.cpp with a small VLM answering fixed structured questions, measured against fixtures | planned |

The remote repository is created after milestone 02, so the first push already runs the gates.

## Implemented

- Workspace with six crates across five layers ([hexagon](architecture/hexagon.md), [layers.json](architecture/layers.json)).
- `jacquard-core`: `Outcome`, `Verdict`, `Judgment`, `Vote`, `DraftId`, `SwatchId`, and the module contracts.
- Product contract, glossary, ADR-0000 to ADR-0003, the [design-loops research digest](research/2026-09-28-design-loops/README.md).

## Current Assumptions

- Sonnet 5.5 is both Loom and sole Judge ([ADR-0002](decisions/ADR-0002-sonnet-sole-judge.md)); self-preference is mitigated, not removed.
- Local models run on one machine: RTX 3060 Laptop 6 GB, 15 GB RAM, WSL2. They filter; they never rank.
- Designers agree with each other at Krippendorff α≈0.25 on UI preference, so the owner's Votes, not a population, are the calibration target.

## Unresolved Questions

- License (Nomos uses Apache-2.0; not yet chosen here).
- Whether `jacquard-judge-local` is worth its adapter before milestone 05 shows what the Checks miss.
- The Installation's own contract clauses (its "machine is the narrative" rule) belong to milestone 05.
