# Admission Set

The known-answer set of [ADR-0005](../../docs/decisions/ADR-0005-judge-admission.md). `swatches/` holds five base pages; `defects.toml` holds five deterministic CSS injections, each naming the probe rule it must trip; `set.toml` pairs every base with every defect at one audit viewport, 25 pairs.

`jacquard admission verify` renders both sides of every pair and fails unless the defect's rule passes on the base and fails on the variant. Only then is "the base is better" a known answer rather than an opinion. `jacquard admission run` asks the Judge about every pair in both orders and writes the admission report to `reports/`.

Changing anything here is a verifier change, and it requires re-admission of every Judge configuration.
