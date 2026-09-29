#![no_std]
#![forbid(unsafe_code)]

pub fn one() -> u8 {
    1
}
// std::time::Instant is not allowed here
pub use core::cmp::Ordering;
pub fn nostd() {
    let _ = mystd::x;
}
