extern crate bincode;
extern crate failure;
extern crate rand;
extern crate rayon;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate stdsimd;
extern crate typenum;

pub mod simple;
pub mod znot;

pub type Element = f32;
