extern crate failure;
extern crate fastmatmult;
#[macro_use]
extern crate structopt;

use std::fmt::Display;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

use failure::Error;
use structopt::StructOpt;

use fastmatmult::simple::Matrix;

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(parse(from_os_str))]
    input1: PathBuf,
    #[structopt(parse(from_os_str))]
    input2: PathBuf,
}

fn measure<N: Display, R, F: FnOnce() -> R>(name: N, f: F) -> R {
    let start = Instant::now();
    let result = f();
    let stop = Instant::now();
    let elapsed = stop - start;
    println!("{}: {}.{:03}", name, elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    result
}

fn run() -> Result<(), Error> {
    let opts = Opts::from_args();
    let m1 = Matrix::load(&opts.input1)?;
    let m2 = Matrix::load(&opts.input2)?;

    let simple = measure("simple", || fastmatmult::simple::multiply(&m1, &m2));
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
