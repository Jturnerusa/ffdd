use core::fmt;
use std::{fs, iter, path::PathBuf};

use clap::Parser;
use memmap2::{Mmap, MmapMut, MmapOptions};

#[derive(Debug)]
struct Error {
    message: String,
    source: Option<Box<dyn std::error::Error>>,
}

#[derive(Clone, Debug, clap::Parser)]
struct Args {
    #[arg(long)]
    in_file: PathBuf,
    #[arg(long)]
    out_file: PathBuf,
    #[arg(long)]
    block_size: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.source.as_ref() {
            Some(source) => Some(&**source),
            None => None,
        }
    }
}

fn main() {
    match run() {
        Ok(()) => (),
        Err(e) => match &e.source {
            Some(source) => eprintln!("{e}: caused by: {source}"),
            None => eprintln!("{e}"),
        },
    }
}

fn run() -> Result<(), Error> {
    let args = Args::parse();

    let infile = fs::OpenOptions::new()
        .read(true)
        .write(false)
        .open(args.in_file.as_path())
        .map_err(|e| Error {
            message: "failed to open in-fil".to_string(),
            source: Some(Box::new(e)),
        })?;

    let outfile = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(args.out_file.as_path())
        .map_err(|e| Error {
            message: "failed to open out-file".to_string(),
            source: Some(Box::new(e)),
        })?;

    let indata = unsafe {
        Mmap::map(&infile).map_err(|e| Error {
            message: "failed to mmap in-file".to_string(),
            source: Some(Box::new(e)),
        })?
    };

    let mut outdata = unsafe {
        MmapOptions::new()
            .len(indata.len())
            .map_mut(&outfile)
            .map_err(|e| Error {
                message: "failed to mmap out-file".to_string(),
                source: Some(Box::new(e)),
            })?
    };

    let mut bytes_read = 0usize;
    let mut bytes_written = 0usize;

    for (i, o) in indata
        .chunks(args.block_size)
        .zip(outdata.chunks_mut(args.block_size))
    {
        bytes_read += i.len();

        match (i.len(), o.len()) {
            (a, b) if a == b => {
                if i != o {
                    o.copy_from_slice(i);
                    bytes_written += b;
                }
            }
            (a, b) if a > b => {
                if i[..b] != o[..b] {
                    o[..b].copy_from_slice(&i[..b]);
                }
            }
            (a, b) if a < b => {
                if i[..a] != o[..a] {
                    o[..a].copy_from_slice(&i[..a]);
                }
            }
            _ => (),
        }

        eprint!(
            "MB read/written: {}/{}\r",
            (bytes_read / 1024usize.pow(2)),
            (bytes_written / 1024usize.pow(2))
        );
    }

    eprintln!(
        "MB read: {}\nMB written: {}",
        (bytes_read / 1024usize.pow(2)),
        (bytes_written / 1024usize.pow(2))
    );

    Ok(())
}
