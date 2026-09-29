# Glossary

One concept, one name. A term here is used exactly and never replaced by a synonym.

| Term | Meaning |
| --- | --- |
| **Draft** | Compiled design intent for one surface: the constraints, tokens, content and audit viewports, authored in typed Rust and consumed as an inert artifact. After the weaving draft, the notation that tells a loom which threads lift. |
| **Loom** | A generator. It weaves a Swatch from a Draft and a seed. It proposes; it never verifies. |
| **Swatch** | One candidate interface woven from a Draft, identified by the content hash of its source. |
| **Render** | The screenshots and diagnostics of a Swatch at the audit viewports. What Judges and owners see. |
| **Check** | A deterministic examination of a Swatch or Render by a registered command. The only kind of verdict that gates. |
| **Outcome** | What one run of a Check reports: `Passed`, `Failed`, `NotRun`, `Interrupted`, `Inconclusive`, `EnvironmentUnavailable`. |
| **Receipt** | The record of one Check run: command, revision, toolchain, Outcome, output hash. Written by tooling, never by hand. |
| **Judge** | A model that returns a Judgment. Admitted by a known-answer set; never a gate. |
| **Judgment** | A Judge's pairwise verdict on two Renders in a stated order. |
| **Vote** | The owner's blind pairwise preference. The calibration target for Judges. |
| **Ranking** | A Bradley–Terry estimate over Judgments and Votes with intervals and a comparison floor. Relative, never absolute. |
| **Cascade** | The ordered tiers: render gate, Checks, structured local filter, Judge, Votes. Only survivors move on. |
| **Counterexample** | A Swatch wrongly accepted or rejected, kept as a fixture with its provenance. |
| **Product** | An interface Jacquard builds and ships, with its own contract and Checks. The Installation is the first. |
| **Audit viewports** | 2560×1440, 1440×900, 1024×768, 390×844. |
