# flatelf

## Library

This crate allows to generate a flat binary with the memory representation of
an ELF. It also allows to generate a FLATELF with the following format:

```text
["FLATELF1"][entry][base_vaddr][flatbin_size][flatbin]
```

## Command

The flatelf command takes an ELF file as input and generates a FLATELF. It
supports two output modes: file and TCP. In file mode, the output is written
into the specified path. In TCP mode, the input file is read with every
connection and the output FLATELF is served at the specified TCP address.

```text
usage: flatelf <input> [tcp:]<output>
```

## Docs

For more details, please see the docs:

```text
cargo doc --open
```
