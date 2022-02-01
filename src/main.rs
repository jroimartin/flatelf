//! flatelf takes an ELF file as input and generates a FLATELF binary.
//!
//! FLATELF files have the following format:
//!
//! `["FLATELF1"][entry][base_vaddr][flatbin_size][flatbin]`

mod elf;
mod endian_read;

use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::net;
use std::num;
use std::path::Path;
use std::process;
use std::thread;

/// A flatelf specific error.
enum Error {
    /// No LOAD segments.
    NoLoadSegments,

    /// Invalid file or memory offset.
    InvalidOffset,

    /// Invalid type conversion.
    InvalidTypeConversion,

    /// ELF parsing error.
    Elf(elf::Error),

    /// IO operation error.
    Io(io::Error),
}

impl From<elf::Error> for Error {
    fn from(err: elf::Error) -> Error {
        Error::Elf(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<num::TryFromIntError> for Error {
    fn from(_err: num::TryFromIntError) -> Error {
        Error::InvalidTypeConversion
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoLoadSegments => write!(f, "no LOAD segments"),
            Error::InvalidOffset => write!(f, "invalid file or memory offset"),
            Error::InvalidTypeConversion => {
                write!(f, "invalid type conversion")
            }
            Error::Elf(err) => write!(f, "ELF error: {}", err),
            Error::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 3 {
        eprintln!("usage: flatelf <input> [tcp:]<output>");
        process::exit(2);
    }

    let input = &args[1];
    let output = &args[2];

    let result = if let Some(listen_addr) = output.strip_prefix("tcp:") {
        serve(Path::new(input), listen_addr)
    } else {
        write_file(Path::new(input), Path::new(output))
    };

    if let Err(err) = result {
        eprintln!("error: {}", err);
        process::exit(1);
    };
}

/// Write output to disk.
fn write_file<P: AsRef<Path>>(
    input_file: P,
    output_file: P,
) -> Result<(), Error> {
    let flatelf = generate_flatelf(input_file)?;
    fs::write(output_file, flatelf)?;
    Ok(())
}

/// Serve output via TCP socket.
fn serve<P: AsRef<Path>, A: net::ToSocketAddrs>(
    input_file: P,
    listen_addr: A,
) -> Result<(), Error> {
    let listener = net::TcpListener::bind(listen_addr)?;

    println!("Listening on {}", listener.local_addr()?);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let input_file = input_file.as_ref().to_path_buf();
                thread::spawn(move || {
                    handle_connection(stream, input_file).unwrap_or_else(
                        |err| {
                            eprintln!("Connection handler error: {}", err);
                        },
                    )
                });
            }
            Err(err) => {
                eprintln!("TCP error: {:?}", err);
            }
        }
    }

    Ok(())
}

/// Handle incoming connection.
fn handle_connection<P: AsRef<Path>>(
    mut stream: net::TcpStream,
    input_file: P,
) -> Result<(), Error> {
    let flatelf = generate_flatelf(input_file)?;

    println!(
        "New connection from {}, sending {} bytes",
        stream.peer_addr()?,
        flatelf.len()
    );

    stream.write_all(&flatelf)?;
    stream.shutdown(net::Shutdown::Both)?;
    Ok(())
}

/// Generates a FLATELF from an input ELF.
fn generate_flatelf<P: AsRef<Path>>(input_file: P) -> Result<Vec<u8>, Error> {
    // Read input file.
    let input_data = fs::read(input_file)?;

    // Get LOAD program headers.
    let elf = elf::Elf::new(&input_data)?;
    let mut load_phdrs = elf
        .phdrs()
        .iter()
        .filter(|phdr| phdr.ptype() == elf::PT_LOAD)
        .copied()
        .collect::<Vec<elf::Phdr>>();

    // Sort LOAD program headers by vaddr.
    load_phdrs.sort_by_key(|phdr| phdr.vaddr());

    // The base address of the flat binary is the virtual address of the first
    // LOAD segment.
    let base_vaddr = load_phdrs.first().ok_or(Error::NoLoadSegments)?.vaddr();

    // Calculate the size of the flat binary.
    let last_phdr = load_phdrs.last().ok_or(Error::NoLoadSegments)?;
    let end_vaddr = last_phdr
        .vaddr()
        .checked_add(last_phdr.memsz())
        .ok_or(Error::InvalidOffset)?;
    let flatbin_size: usize = end_vaddr
        .checked_sub(base_vaddr)
        .ok_or(Error::InvalidOffset)?
        .try_into()?;

    // Generate the flat binary.
    let mut flatbin = vec![0u8; flatbin_size];
    for phdr in &load_phdrs {
        let size: usize = phdr.filesz().try_into()?;

        // Get segment's contents.
        let src_start: usize = phdr.offset().try_into()?;
        let src_end: usize =
            src_start.checked_add(size).ok_or(Error::InvalidOffset)?;
        let src = input_data
            .get(src_start..src_end)
            .ok_or(Error::InvalidOffset)?;

        // Get flatbin's destination.
        let dst_start: usize = phdr
            .vaddr()
            .checked_sub(base_vaddr)
            .ok_or(Error::InvalidOffset)?
            .try_into()?;
        let dst_end: usize =
            dst_start.checked_add(size).ok_or(Error::InvalidOffset)?;
        let dst = flatbin
            .get_mut(dst_start..dst_end)
            .ok_or(Error::InvalidOffset)?;

        // Copy segment's contents at the corresponding flatbin location.
        dst.copy_from_slice(src);
    }

    // Generate flatelf.
    let mut flatelf = Vec::new();
    flatelf.extend(b"FLATELF1");
    flatelf.extend(&elf.entry().to_le_bytes());
    flatelf.extend(&base_vaddr.to_le_bytes());
    flatelf.extend(&u64::try_from(flatbin_size)?.to_le_bytes());
    flatelf.extend(&flatbin);

    Ok(flatelf)
}
