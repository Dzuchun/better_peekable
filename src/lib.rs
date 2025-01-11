#![no_std]

#[cfg(any(test, kani))]
extern crate alloc;

pub mod iterator;

mod dequeue;
