#![feature(nll)]
extern crate bincode;
extern crate failure;
#[macro_use] // tuplify macro ‒ abused somewhere else, but who cares
extern crate faster;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate smallvec;
extern crate typenum;

pub mod simd;
pub mod simple;
pub mod znot;

pub type Element = f32;
