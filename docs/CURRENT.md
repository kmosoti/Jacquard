# Current Project State

## Active Work

**Milestone 04, semantic kernels**, on `milestone/04-semantic-kernels`: Bradley–Terry ranking with bootstrap intervals and the semantic-mutant corpus, 14 wrong behaviors each caught by a named test. Next: milestone 05, the Installation as the first Product.

## Milestones

| Nn | Name | Lands | Status |
| --- | --- | --- | --- |
| 01 | architecture-foundation | workspace, layers, contract, glossary, ADR-0000..0003, research digest | landed |
| 02 | verification-foundation | `jacquard-xtask`: `check-layers`, `check-core-purity`, `check-trust-boundary`, check registry with receipts, fixture negative controls, CI | landed |
| 03 | factory-ports | Loom, Renderer, Checker, Judge and Ledger ports and their first adapters; Judge admission set | landed |
| 04 | semantic-kernels | Bradley–Terry ranking with reference implementation and metamorphic relations; semantic-mutant corpus | landed |
| 05 | product-installation | the Installation imported as the first Product from a clean snapshot; its own contract and Checks | planned |
| 06 | local-filter | llama.cpp with a small VLM answering fixed structured questions, measured against fixtures | planned |

The remote repository is created after milestone 02, so the first push already runs the gates. A scheduled generic `cargo mutants` calibration is not set up; the semantic corpus runs on every push.

## Implemented

- Workspace across five layers, members globbed by layer ([hexagon](architecture/hexagon.md), [layers.json](architecture/layers.json)).
- `jacquard-core`: the vocabulary types, viewports, the five probe rules (overflow, clipped text, overlapping text, contrast, target size), and the admission kernel.
- Ports: Loom, Renderer, Checker, Judge, Ledger. Adapters: `jacquard-ledger-fs`, `jacquard-renderer-chrome`, `jacquard-judge-claude`. The Loom and a process Checker arrive with the first Product.
- `jacquard render | compare [--ledger] | rank | admission verify | admission run`.
- Ranking (ADR-0006): Bradley–Terry with a phantom prior, MM estimation agreeing with an independent Newton reference to 1e-8, 95% bootstrap intervals, the floor of 15.
- The semantic-mutant corpus (ADR-0007): 14 wrong behaviors, all caught; its first run found a Swatch-identity test that could not fail, since strengthened.
- The admission set: 25 known-answer pairs, all verified. **Sonnet 5.5 admitted** on 2026-09-28: rubric `5e4b17a93faf026b`, 25 of 25 correct, 0 flipped, all 50 Judgments answered by `claude-sonnet-5-5` ([report](../verification/admission/reports/5e4b17a93faf026b-claude-sonnet-5-5.ndjson)).
- `jacquard-xtask`: five gates (layers, purity, trust boundary, receipts, semantic mutants), the check registry with ten checks, receipts, CI.
- Product contract, glossary, ADR-0000 to ADR-0007, the design-loops research digest.

## Known Gaps

- The admission set tests mechanical defects only, and Sonnet 5.5 scored at the ceiling. It proves the Judge can see breakage consistently across order; it says nothing about taste. A harder set (subtle hierarchy, spacing and density differences with a known answer) and the owner's Votes are what would measure that.
- The alias `sonnet` answers as `claude-sonnet-5`; design-workflow agents must pin `claude-sonnet-5-5`, which the Agent tool's model option cannot do.
- The judge depends on the owner's Claude CLI login; each Judgment costs a few US cents.

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
