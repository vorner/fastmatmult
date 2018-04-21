extern crate bincode;
extern crate failure;
extern crate faster;
extern crate rand;
extern crate rayon;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate typenum;

pub mod simd;
pub mod simple;
pub mod znot;

pub type Element = f32;
