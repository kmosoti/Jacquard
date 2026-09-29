# ADR-0002: Sonnet 5.5 Is the Sole Judge and the Design-Workflow Model

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** Owner decision of 2026-09-28: "use Sonnet 5.5 exclusively for the design workflow" and "for now we will keep Sonnet as the sole judge". Evidence from the [research digest](../research/2026-09-28-design-loops/README.md).

## Context

The design workflow is the point of the repository. The owner chose one model for every agent in it: weaving, critique, judging, visual review and mockups. The same model then generates and judges, and the digest records that self-preference bias is uncorrelated with capability and that order alone moves pairwise verdicts by more than ten percent.

## Decision

1. Every agent in the design workflow runs Claude Sonnet 5.5. No other model weaves, critiques or judges. Repository tooling and documentation work is outside the design workflow.
2. Sonnet 5.5 is the sole Judge. Its Judgments are pairwise, presented in both orders, on rendered evidence only, against a decomposed multi-dimension rubric (the mitigation that cut self-preference by 31.5% in the cited study).
3. Because Judgments never gate ([ADR-0001](ADR-0001-accept-work-on-executable-evidence.md)), the residual bias affects which accepted Swatch the owner sees first, not what is accepted.
4. The Judge's admission set includes pairs where the verified-worse Swatch was woven by Sonnet, so self-preference is measured, not assumed away.
5. Small local models may answer fixed structured questions as a filter tier (milestone 06). They never produce a Judgment.

## Consequences

Easier: one model configuration to admit and re-admit; no cross-vendor plumbing. Harder: the same blind spots in Loom and Judge; the owner's Votes are the only independent signal, so they are collected from the start and a personalised preference model is the planned second opinion.

## Validation

Revisited if admission shows a position-flip rate or self-preference the rubric cannot bring below the admission threshold set in milestone 03, or when the owner says so.
