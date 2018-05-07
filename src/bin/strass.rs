#![feature(test)]

extern crate failure;
extern crate fastmatmult;
#[macro_use]
extern crate structopt;
extern crate test;
extern crate typenum;

use std::fmt::Display;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

use failure::Error;
use structopt::StructOpt;
use typenum::U256;

use fastmatmult::simple::Matrix;
use fastmatmult::znot::{Matrix as ZMat, RayonDistribute, SimdMultiplyAdd};

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(parse(from_os_str))]
    input1: PathBuf,
    #[structopt(parse(from_os_str))]
    input2: PathBuf,
    /// Skip over some expensive computations.
    ///
    /// This is to be able to measure somewhat larger inputs, so skipping the really slow ones
    /// helps.
    #[structopt(short = "c", long = "cheap")]
    cheap: bool,

    /// Run only the simple multiplication.
    #[structopt(short = "s", long = "simple-only")]
    simple_only: bool,
}

fn measure<N: Display, R, F: FnOnce() -> R>(name: N, f: F) -> R {
    let start = Instant::now();
    let result = test::black_box(f());
    let stop = Instant::now();
    let elapsed = stop - start;
    println!("{}: {}.{:03}", name, elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    result
}

fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    let m1 = Matrix::load(&opts.input1)?;
    let m2 = Matrix::load(&opts.input2)?;

    measure("strassen-256", || {
        let a_z = ZMat::<U256>::from(&m1);
        let b_z = ZMat::<U256>::from(&m2);
        let r_z = fastmatmult::znot::strassen::<_, RayonDistribute<U256>, SimdMultiplyAdd>(&a_z, &b_z);
        Matrix::from(&r_z)
    });

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
