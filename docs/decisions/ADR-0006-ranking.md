# ADR-0006: Ranking

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** [Product contract](../PRODUCT-CONTRACT.md) clause 4 and the [research digest](../research/2026-09-28-design-loops/README.md): Chatbot Arena's Bradley–Terry with intervals, Design Arena's floor of fifteen comparisons, and UI2Code^N's finding that relative rankings beat absolute scores.

## Context

Judgments and Votes are pairwise. The owner needs an order over Swatches, with a statement of how sure it is, and a refusal to rank a Swatch that has barely been compared. The estimate must be a pure function of the recorded comparisons, so that the same records always give the same ranking.

## Decision

1. **Model.** Bradley–Terry: Swatch $i$ has strength $p_i > 0$ and beats $j$ with probability $p_i / (p_i + p_j)$.
2. **Data.** A comparison is two Swatches in a presentation order and a verdict. A win scores one to the winner; a tie scores one half to each. A both-order comparison is two comparisons. Comparisons are aggregated into integer half-unit counts before anything else, so the estimate cannot depend on the order the records arrive in.
3. **Prior.** Every Swatch plays one virtual win and one virtual loss against a phantom of fixed strength 1. This keeps every strength finite when one Swatch has won every comparison, anchors the scale (the phantom is 1), and connects the comparison graph.
4. **Estimator.** The minorization–maximization iteration of Hunter (2004), $p_i \leftarrow W_i / \sum_j n_{ij} / (p_i + p_j)$, run until no strength changes by more than one part in $10^{12}$ or 10,000 iterations pass. Implemented in `jacquard-core`, with no floating-point function beyond the four operations.
5. **Intervals.** A 95% percentile bootstrap over the comparisons, 200 resamples, drawn by a seeded SplitMix64 generator over the canonically sorted comparisons. The seed is an input; the same records and seed give the same intervals everywhere.
6. **Floor.** A Swatch with fewer than 15 comparisons is estimated but **unranked**, and says so.
7. **Output.** Strengths relative to the phantom, their intervals, comparison counts, and ranks for ranked Swatches only. A display rating of $400 \log_{10} p$ is formatting, done outside the core.

### Relations Stated Before the Code

The kernel is tested against an independently written reference (Newton's method on the log-likelihood, using the host's logarithms) and these relations:

- Permuting the comparison records does not change the ranking, bit for bit.
- Relabeling the Swatches permutes the output accordingly.
- If $i$ and $j$ have the same record against each other and against everything else, their strengths are equal.
- Adding a win for $i$ over $j$ does not lower $p_i / p_j$.
- A tie between $i$ and $j$ never reverses their order.

## Consequences

Easier: one number per Swatch with an honest interval and an honest "not yet". Harder: 200 resamples per ranking; fine at the sizes a design session produces.

## Validation

Falsified if the same records give two rankings, or if a Swatch below the floor is given a rank.
