extern crate bincode;
extern crate failure;
extern crate fastmatmult;
extern crate itertools;
extern crate rand;
#[macro_use]
extern crate structopt;

use std::path::{Path, PathBuf};
use std::process;

use failure::Error;
use itertools::Itertools;
use rand::Rng;
use structopt::StructOpt;

use fastmatmult::simple::Matrix;

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "generate")]
    Generate {
        width: usize,
        height: usize,
        #[structopt(parse(from_os_str))]
        file: PathBuf,
    },
    #[structopt(name = "show")]
    Show {
        #[structopt(parse(from_os_str))]
        file: PathBuf,
    },
}

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    command: Command,
}

fn generate(width: usize, height: usize, file: &Path) -> Result<(), Error> {
    let mut matrix = Matrix::sized(width, height);
    let mut rng = rand::thread_rng();
    for x in 0..width {
        for y in 0..height {
            matrix[(x, y)] = rng.gen_range(0., 10.);
        }
    }
    matrix.store(file)?;
    Ok(())
}

fn show(file: &Path) -> Result<(), Error> {
    let matrix = Matrix::load(file)?;
    for row in matrix.rows() {
        println!("{:.3}", row.iter().format(" "));
    }
    Ok(())
}

fn main() {
    let opts = Opts::from_args();
    let result = match opts.command {
        Command::Generate { width, height, file } => generate(width, height, &file),
        Command::Show { file } => show(&file),
    };
    if let Err(e) = result {
        eprintln!("{}", e);
        process::exit(1);
    }
}
