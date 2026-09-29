use fx::{double, half};

#[test]
fn doubles() {
    assert_eq!(double(3), 6);
}

#[test]
fn halves() {
    assert_eq!(half(8), 4);
}

#[test]
fn broken() {
    assert_eq!(half(3), 2);
}
