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
use typenum::{U1, U2, U4, U8, U16, U32, U64, U128, U256, U512, U1024, Unsigned};

use fastmatmult::simple::Matrix;
use fastmatmult::znot::{
    Distribute, DontDistribute, FragMultiplyAdd, Matrix as ZMat, RayonDistribute, SimdMultiplyAdd,
    SimpleMultiplyAdd
};

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
}

fn measure<N: Display, R, F: FnOnce() -> R>(name: N, f: F) -> R {
    let start = Instant::now();
    let result = test::black_box(f());
    let stop = Instant::now();
    let elapsed = stop - start;
    println!("{}: {}.{:03}", name, elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    result
}

fn block_inner<Dist, Mult, Frag>(suffix: &str, a: &Matrix, b: &Matrix, expected: Option<&Matrix>)
where
    Dist: Distribute,
    Frag: Unsigned + Default,
    Mult: FragMultiplyAdd,
{
    let r = measure(format!("recursive{}-{}", suffix, Frag::USIZE), || {
        let a_z = ZMat::<Frag>::from(a);
        let b_z = ZMat::<Frag>::from(b);
        let r_z = measure(format!("recursive-inner{}-{}", suffix, Frag::USIZE), || {
            fastmatmult::znot::multiply::<_, Dist, Mult>(&a_z, &b_z)
        });
        Matrix::from(&r_z)
    });

    if let Some(expected) = expected {
        assert_eq!(expected, &r);
    }
}

fn block<Frag>(a: &Matrix, b: &Matrix, expected: Option<&Matrix>)
where
    Frag: Unsigned + Default,
{
    if a.width() < Frag::USIZE {
        return;
    }
    block_inner::<DontDistribute, SimpleMultiplyAdd, Frag>("", a, b, expected);
    block_inner::<RayonDistribute<Frag>, SimpleMultiplyAdd, Frag>("-paral", a, b, expected);
    block_inner::<RayonDistribute<U256>, SimpleMultiplyAdd, Frag>("-paral-cutoff", a, b, expected);
    if Frag::USIZE >= 4 {
        block_inner::<DontDistribute, SimdMultiplyAdd, Frag>("-simd", a, b, None);
        block_inner::<RayonDistribute<Frag>, SimdMultiplyAdd, Frag>("-simd-paral", a, b, None);
        block_inner::<RayonDistribute<U256>, SimdMultiplyAdd, Frag>(
            "-simd-paral-cutoff",
            a,
            b,
            None
        );
    }
    if Frag::USIZE >= 32 {
        measure(format!("strassen-{}", Frag::USIZE), || {
            let a_z = ZMat::<Frag>::from(a);
            let b_z = ZMat::<Frag>::from(b);
            let r_z = measure(format!("strassen-inner-{}", Frag::USIZE), || {
                fastmatmult::znot::strassen::<_, RayonDistribute<Frag>, SimdMultiplyAdd>(&a_z, &b_z)
            });
            Matrix::from(&r_z)
        });
    }
}

fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    let m1 = Matrix::load(&opts.input1)?;
    let m2 = Matrix::load(&opts.input2)?;

    let simple = if opts.cheap {
        None
    } else {
        let simple = measure("simple", || fastmatmult::simple::multiply(&m1, &m2));
        let col_cp = measure("colcp", || fastmatmult::simple::multiply_col_cp(&m1, &m2));
        assert_eq!(simple, col_cp);
        Some(simple)
    };
    let simple = simple.as_ref();

    let _simd = measure("simd", || fastmatmult::simd::multiply(&m1, &m2));
    // Not checking equality, because simd does slightly different results due to reordering of the
    // summing

    if !opts.cheap {
        block::<U1>(&m1, &m2, simple);
        block::<U2>(&m1, &m2, simple);
        block::<U4>(&m1, &m2, simple);
        block::<U8>(&m1, &m2, simple);
        block::<U16>(&m1, &m2, simple);
        block::<U32>(&m1, &m2, simple);
    }
    block::<U64>(&m1, &m2, simple);
    block::<U128>(&m1, &m2, simple);
    block::<U256>(&m1, &m2, simple);
    block::<U512>(&m1, &m2, simple);
    block::<U1024>(&m1, &m2, simple);

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
