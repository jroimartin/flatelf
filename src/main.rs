//! The flatelf command takes an ELF file as input and generates a FLATELF. It
//! supports two output modes: file and TCP. In file mode, the output is
//! written into the specified path. In TCP mode, the input file is read with
//! every connection and the output FLATELF is served at the specified TCP
//! address.
//!
//! ```text
//! usage: flatelf <input> [tcp:]<output>
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

    result.unwrap_or_else(|err| {
        eprintln!("error: {}", err);
        process::exit(1);
    });
}

/// Write output to disk.
fn write_file<P: AsRef<Path>>(
    input_file: P,
    output_file: P,
) -> Result<(), Error> {
    let data = fs::read(input_file)?;
    let flatelf = FlatElf::new(&data)?.flatelf()?;

    println!(
        "Writing {} bytes to {}",
        flatelf.len(),
        output_file.as_ref().display()
    );

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

/// Handle incoming connection.
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
