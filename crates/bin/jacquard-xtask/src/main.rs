//! `cargo xtask <command>`: every check, generator and reproducer that
//! produces a record lives here, in Rust, never in a shell or Python script.
//! Planned for milestone 02: `check-layers`, `check-core-purity`,
//! `check-trust-boundary`, `checks --profile fast`, `receipts validate`.

fn main() {
    let command = std::env::args().nth(1).unwrap_or_default();
    eprintln!("xtask: no gates are registered yet (milestone 02); got `{command}`");
    std::process::exit(2);
}
