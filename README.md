# flatelf

## Library

This crate allows to generate a flat binary with the memory representation of
an ELF. It also allows to generate a FLATELF with the following format:

```text
["FLATELF1"][entry][base_vaddr][flatbin_size][flatbin]
```

## Command

The flatelf command takes an ELF file as input and generates a flat binary.

It supports the following output modes:

- **flatelf**: the input ELF is converted into a FLATELF and written to disk.
- **flatbin**: the input ELF is converted into a flat binary and written to
  disk. Its base virtual address and entrypoint are print to stdout for easy
  parsing.
- **tcp**: the input file is read with every connection and converted into a
  FLATELF which is served at the specified TCP address.

Usage:

```text
usage: flatelf <input> <mode:output>
modes:
  flatelf:/path/to/file
  flatbin:/path/to/file
  tcp:127.0.0.1:1234
```

## Docs

For more details, please see the docs:

```text
cargo doc --open
```
