//! The flatelf command takes an ELF file as input and generates a FLATELF. It
//! supports two output modes: file and TCP. In file mode, the output is
//! written into the specified path. In TCP mode, the input file is read with
//! every connection and the output FLATELF is served at the specified TCP
//! address.
//!
//! ```text
//! usage: flatelf <input> <mode:output>
//! modes:
//!   flatelf:/path/to/file
//!   flatbin:/path/to/file
//!   tcp:127.0.0.1:1234
//! ```

use std::env;
use std::fs;
use std::io::Write;
use std::net;
use std::path::Path;
use std::process;
use std::thread;

use flatelf::error::Error;
use flatelf::FlatElf;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 3 {
        usage();
    }

    let input = &args[1];
    let mode_output = &args[2];

    let parts = mode_output.split(':').collect::<Vec<&str>>();
    if parts.len() != 2 {
        usage();
    }

    let mode = parts[0];
    let output = parts[1];

    let result = match mode {
        "flatelf" => write_flatelf(Path::new(input), Path::new(output)),
        "flatbin" => write_flatbin(Path::new(input), Path::new(output)),
        "tcp" => serve(Path::new(input), output),
        _ => usage(),
    };

    result.unwrap_or_else(|err| {
        eprintln!("error: {}", err);
        process::exit(1);
    });
}

/// Writes flatelf to disk.
fn write_flatelf<P: AsRef<Path>>(
    input_file: P,
    output_file: P,
) -> Result<(), Error> {
    let data = fs::read(input_file)?;
    let flatelf = FlatElf::new(&data)?.flatelf()?;

    fs::write(output_file, flatelf)?;
    Ok(())
}

/// Writes flatbin to disk and prints "base_vaddr entry" to stdou.
fn write_flatbin<P: AsRef<Path>>(
    input_file: P,
    output_file: P,
) -> Result<(), Error> {
    let data = fs::read(input_file)?;
    let flatelf = FlatElf::new(&data)?;

    println!("{:#x} {:#x}", flatelf.base_vaddr(), flatelf.entry());

    fs::write(output_file, flatelf.flatbin())?;
    Ok(())
}

/// Serves output via TCP socket.
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
                            eprintln!("Connection error: {}", err);
                        },
                    )
                });
            }
            Err(err) => {
                eprintln!("TCP error: {}", err);
            }
        }
    }

    Ok(())
}

/// Handles incoming connection.
fn handle_connection<P: AsRef<Path>>(
    mut stream: net::TcpStream,
    input_file: P,
) -> Result<(), Error> {
    let data = fs::read(input_file)?;
    let flatelf = FlatElf::new(&data)?.flatelf()?;

    println!(
        "New connection from {}, sending {} bytes",
        stream.peer_addr()?,
        flatelf.len()
    );

    stream.write_all(&flatelf)?;
    stream.shutdown(net::Shutdown::Both)?;
    Ok(())
}

/// Prints usage message to stderr and exit with return code 2.
fn usage() -> ! {
    eprintln!(
        r#"usage: flatelf <input> <mode:output>
modes:
  flatelf:/path/to/file
  flatbin:/path/to/file
  tcp:127.0.0.1:1234"#
    );
    process::exit(2);
}
