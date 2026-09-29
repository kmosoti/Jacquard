//! The filesystem Ledger: Receipts, Judgments and Votes appended as NDJSON
//! under `verification/`, one file per kind, never rewritten. A record is a
//! record of what ran or what was chosen; it is committed, and it is not an
//! artifact.
#![forbid(unsafe_code)]
