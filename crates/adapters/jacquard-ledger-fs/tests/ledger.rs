//! Append-only: records come back in order, a missing stream is empty, and
//! a multi-line record or a path-like stream name is refused.

use jacquard_ledger_fs::FsLedger;
use jacquard_ports::Ledger;

#[test]
fn records_append_and_read_back_in_order() {
    let dir = std::env::temp_dir().join(format!("jacquard-ledger-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ledger = FsLedger::new(&dir);
    assert_eq!(ledger.read("votes").expect("read"), Vec::<String>::new());
    ledger.append("votes", r#"{"n":1}"#).expect("append");
    ledger.append("votes", r#"{"n":2}"#).expect("append");
    ledger.append("judgments", r#"{"n":3}"#).expect("append");
    assert_eq!(
        ledger.read("votes").expect("read"),
        [r#"{"n":1}"#, r#"{"n":2}"#]
    );
    assert!(ledger.append("votes", "a\nb").is_err());
    for bad in ["", "../escape", "a/b", ".hidden", "Votes"] {
        assert!(ledger.append(bad, "{}").is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
