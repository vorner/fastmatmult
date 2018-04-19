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
use typenum::{U1, U2, U4, U8, U16, U32, U64, U128, U256, Unsigned};

use fastmatmult::simple::Matrix;
use fastmatmult::znot::{DontDistribute, Matrix as ZMat, RayonDistribute};

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(parse(from_os_str))]
    input1: PathBuf,
    #[structopt(parse(from_os_str))]
    input2: PathBuf,
}

fn measure<N: Display, R, F: FnOnce() -> R>(name: N, f: F) -> R {
    let start = Instant::now();
    let result = test::black_box(f());
    let stop = Instant::now();
    let elapsed = stop - start;
    println!("{}: {}.{:03}", name, elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    result
}

fn block<Frag: Unsigned + Default>(a: &Matrix, b: &Matrix, expected: &Matrix) {
    let r = measure(format!("recursive-{}", Frag::USIZE), || {
        let a_z = ZMat::<Frag>::from(a);
        let b_z = ZMat::<Frag>::from(b);
        let r_z = measure(format!("recursive-inner-{}", Frag::USIZE), || {
            fastmatmult::znot::multiply::<_, DontDistribute>(&a_z, &b_z)
        });
        Matrix::from(&r_z)
    });

    assert_eq!(expected, &r);

    let r = measure(format!("recursive-paral-{}", Frag::USIZE), || {
        let a_z = ZMat::<Frag>::from(a);
        let b_z = ZMat::<Frag>::from(b);
        let r_z = measure(format!("recursive-paral-inner-{}", Frag::USIZE), || {
            fastmatmult::znot::multiply::<_, RayonDistribute<Frag>>(&a_z, &b_z)
        });
        Matrix::from(&r_z)
    });

    assert_eq!(expected, &r);

    let r = measure(format!("recursive-paral-cutoff-{}", Frag::USIZE), || {
        let a_z = ZMat::<Frag>::from(a);
        let b_z = ZMat::<Frag>::from(b);
        let r_z = measure(format!("recursive-paral-cutoff-inner-{}", Frag::USIZE), || {
            fastmatmult::znot::multiply::<_, RayonDistribute<U256>>(&a_z, &b_z)
        });
        Matrix::from(&r_z)
    });

    assert_eq!(expected, &r);
}

fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    let m1 = Matrix::load(&opts.input1)?;
    let m2 = Matrix::load(&opts.input2)?;

    let simple = measure("simple", || fastmatmult::simple::multiply(&m1, &m2));

    block::<U1>(&m1, &m2, &simple);
    block::<U2>(&m1, &m2, &simple);
    block::<U4>(&m1, &m2, &simple);
    block::<U8>(&m1, &m2, &simple);
    block::<U16>(&m1, &m2, &simple);
    block::<U32>(&m1, &m2, &simple);
    block::<U64>(&m1, &m2, &simple);
    block::<U128>(&m1, &m2, &simple);

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
