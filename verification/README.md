# Verification

- [`checks.toml`](checks.toml) is the check registry: every check that can produce a receipt, with what a pass establishes and what it does not.
- [`trust-boundary.toml`](trust-boundary.toml) is the list of protected paths and the escape-hatch patterns ([ADR-0001](../docs/decisions/ADR-0001-accept-work-on-executable-evidence.md)).
- [`receipts/`](receipts/) holds committed receipts, one NDJSON line per run, written only by `cargo xtask receipts record`. A receipt records the command, the revision it ran on, whether the tree was dirty, the toolchain, the outcome, the exit code and a hash of the output. It is not signed: it proves the record's structure, and that the recorder ran the command, not more.

`cargo xtask checks --profile fast` runs the fast set and writes its receipts under `target/verification/receipts/`, which is not committed. Record a receipt for the repository with:

```sh
cargo xtask receipts record <check-id> --out verification/receipts/<file>.ndjson -- <command>
```

Every gate prints a stable code per failure and is tested against fixtures under `tests/fixtures/<gate>/`, one or more cases per code:

| Gate | Prefix | Codes |
| --- | --- | --- |
| `check-layers` | `FORBIDDEN` | `unassigned-crate`, `stale-assignment`, `unknown-layer`, `forbidden-edge` |
| `check-core-purity` | `IMPURE` | `core-dependency-denied`, `core-build-script`, `core-not-no-std`, `core-unsafe-not-forbidden`, `core-uses-std` |
| `check-trust-boundary` | `UNDECLARED` | `undeclared-oracle-change`, `mixed-oracle-and-implementation`, `escape-hatch-added`, `trailer-without-change`, `unknown-trust-boundary-class`; an unresolvable base is `gate-could-not-run` |
| `mutants semantic` | `SURVIVED` | `mutant-survived`, `mutant-not-applicable`, `mutant-test-missing`, `mutant-baseline-failed`, `mutant-unviable` |
| `receipts validate` | `REJECTED` | `receipt-malformed`, `receipt-unregistered-check`, `receipt-command-mismatch`, `receipt-passed-without-evidence`, `receipt-unknown-outcome` |

A gate that cannot run prints `ERROR [gate-could-not-run]` and exits 2.
