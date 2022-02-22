//! The flatelf command takes an ELF file as input and generates a flat binary.
//!
//! ```text
//! usage: flatelf <input> <output>
//! ```

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use flatelf::error::Error;
use flatelf::FlatElf;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 3 {
        eprintln!("usage: flatelf <input> <output>");
        process::exit(2);
    }
    let input = &args[1];
    let output = &args[2];

    write_flatbin(Path::new(input), Path::new(output)).unwrap_or_else(|err| {
        eprintln!("error: {}", err);
        process::exit(1);
    });
}

/// Writes flatbin to disk and prints its base address, entry point and size to
/// stdout.
fn write_flatbin<P: AsRef<Path>>(
    input_file: P,
    output_file: P,
) -> Result<(), Error> {
    let data = fs::read(input_file)?;
    let flatelf = FlatElf::new(&data)?;

    let flatbin = flatelf.flatbin();

    println!(
        "{:#x} {:#x} {:#x}",
        flatelf.base_vaddr(),
        flatelf.entry(),
        flatbin.len()
    );

    fs::write(output_file, flatbin)?;
    Ok(())
}
