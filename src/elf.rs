//! Minimal ELF parser focused on creating flat binaries.

use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::endian_read::{EndianRead, FromBytes};
use crate::error::Error;

/// CPU word size.
enum Size {
    /// 32-bit CPU.
    Bits32,

    /// 64-bit CPU.
    Bits64,
}

impl TryFrom<u8> for Size {
    type Error = Error;

    fn try_from(size: u8) -> Result<Size, Error> {
        match size {
            1 => Ok(Size::Bits32),
            2 => Ok(Size::Bits64),
            _ => Err(Error::InvalidSize),
        }
    }
}

/// Endianness.
enum Endian {
    /// Little endian.
    Little,

    /// Big endian.
    Big,
}

impl TryFrom<u8> for Endian {
    type Error = Error;

    fn try_from(endian: u8) -> Result<Endian, Error> {
        match endian {
            1 => Ok(Endian::Little),
            2 => Ok(Endian::Big),
            _ => Err(Error::InvalidEndian),
        }
    }
}

/// Size of the ELF identification header.
const EIDENT_SIZE: usize = 0x10;

/// ELF header.
struct Ehdr {
    /// Identifies the object file type.
    _etype: u16,

    /// Specifies the target instruction set architecture.
    _machine: u16,

    /// ELF version.
    version: u32,

    /// Entry point from where the process starts executing.
    entry: u64,

    /// Points to the start of the program header table.
    phoff: u64,

    /// Points to the start of the section header table.
    _shoff: u64,

    /// Flags.
    _flags: u32,

    /// Size of this header.
    _ehsize: u16,

    /// Size of a program header entry.
    phentsize: u16,

    /// Number of entries in the program header table.
    phnum: u16,

    /// Size of a program header table entry.
    _shentsize: u16,

    /// Number of entries in the section header table.
    _shnum: u16,

    /// Index of the section header table entry that contains the section
    /// names.
    _shstrndx: u16,
}

/// [`Phdr::ptype`] value for loadable segments.
pub const PT_LOAD: u32 = 0x1;

/// ELF program header. Only the fields needed to create a flat binary are
/// accessible.
#[derive(Clone, Copy)]
pub struct Phdr {
    /// Segment type.
    ptype: u32,

    /// Segment-dependent flags.
    _flags: u32,

    /// Offset of the segment in the file image.
    offset: u64,

    /// Virtual address of the segment in memory.
    vaddr: u64,

    /// On systems where the physical address is relevant, reserved for
    /// segment's physical address.
    _paddr: u64,

    /// Size in bytes of the segment in the file image.
    filesz: u64,

    /// Size in bytes of the segment in memory.
    memsz: u64,

    /// Alignment of the segment.
    _align: u64,
}

impl Phdr {
    /// Returns the segment type.
    pub fn ptype(&self) -> u32 {
        self.ptype
    }

    /// Returns the offset of the segment in the file image.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the virtual address of the segment in memory.
    pub fn vaddr(&self) -> u64 {
        self.vaddr
    }

    /// Returns the size in bytes of the segment in the file image.
    pub fn filesz(&self) -> u64 {
        self.filesz
    }

    /// Returns the size in bytes of the segment in memory.
    pub fn memsz(&self) -> u64 {
        self.memsz
    }
}

/// Parsed ELF.
pub struct Elf {
    /// Entry point from where the process starts executing.
    entry: u64,

    /// Program headers.
    phdrs: Vec<Phdr>,
}

impl Elf {
    /// Returns a structure representing a parsed ELF. `data` must be a valid
    /// ELF file.
    pub fn new<B: AsRef<[u8]>>(data: &B) -> Result<Elf, Error> {
        Parser::new(data)?.parse()
    }

    /// Returns the entry point from where the process starts executing.
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// Returns the program headers of the parsed ELF.
    pub fn phdrs(&self) -> &[Phdr] {
        &self.phdrs
    }
}

/// ELF Parser.
struct Parser {
    /// CPU word size.
    size: Size,

    /// Endianness.
    endian: Endian,

    /// ELF data.
    data: Cursor<Vec<u8>>,
}

impl Parser {
    /// Returns a new ELF parser. `data` must be a valid ELF file.
    fn new<B: AsRef<[u8]>>(data: &B) -> Result<Parser, Error> {
        let mut data = Cursor::new(data.as_ref().to_vec());

        let mut eident = [0u8; EIDENT_SIZE];
        data.read_exact(&mut eident)?;

        // Check that ELF's magic is "\x7fELF".
        if &eident[..0x4] != b"\x7fELF" {
            return Err(Error::InvalidElfMagic);
        }

        let size = eident[0x4].try_into()?;
        let endian = eident[0x5].try_into()?;

        // Check that ELF's version is 1 (current).
        if eident[0x6] != 1 {
            return Err(Error::InvalidElfVersion);
        }

        // The rest of the fields in the ELF identification are not parsed.
        // They are not used and do not need to be checked. So, the parser can
        // just be returned.
        Ok(Parser { data, size, endian })
    }

    /// Parses the ELF file provided to [`Parser::new`] and returns the
    /// corresponding parsed ELF.
    fn parse(&mut self) -> Result<Elf, Error> {
        let ehdr = self.parse_ehdr()?;

        // Version must be 1 for the original version of ELF.
        if ehdr.version != 1 {
            return Err(Error::InvalidElfVersion);
        }

        let mut phdrs = Vec::new();
        for idx in 0..ehdr.phnum as u64 {
            let entry_offset = idx
                .checked_mul(ehdr.phentsize as u64)
                .ok_or(Error::InvalidOffset)?;
            let phdr_offset = ehdr
                .phoff
                .checked_add(entry_offset)
                .ok_or(Error::InvalidOffset)?;
            let phdr = self.parse_phdr(phdr_offset)?;
            phdrs.push(phdr)
        }

        Ok(Elf {
            entry: ehdr.entry,
            phdrs,
        })
    }

    /// Parses the ELF header.
    fn parse_ehdr(&mut self) -> Result<Ehdr, Error> {
        self.data.seek(SeekFrom::Start(EIDENT_SIZE as u64))?;

        let ehdr = match self.size {
            Size::Bits32 => Ehdr {
                _etype: self.read_val::<u16>()?,
                _machine: self.read_val::<u16>()?,
                version: self.read_val::<u32>()?,
                entry: self.read_val::<u32>()? as u64,
                phoff: self.read_val::<u32>()? as u64,
                _shoff: self.read_val::<u32>()? as u64,
                _flags: self.read_val::<u32>()?,
                _ehsize: self.read_val::<u16>()?,
                phentsize: self.read_val::<u16>()?,
                phnum: self.read_val::<u16>()?,
                _shentsize: self.read_val::<u16>()?,
                _shnum: self.read_val::<u16>()?,
                _shstrndx: self.read_val::<u16>()?,
            },
            Size::Bits64 => Ehdr {
                _etype: self.read_val::<u16>()?,
                _machine: self.read_val::<u16>()?,
                version: self.read_val::<u32>()?,
                entry: self.read_val::<u64>()?,
                phoff: self.read_val::<u64>()?,
                _shoff: self.read_val::<u64>()?,
                _flags: self.read_val::<u32>()?,
                _ehsize: self.read_val::<u16>()?,
                phentsize: self.read_val::<u16>()?,
                phnum: self.read_val::<u16>()?,
                _shentsize: self.read_val::<u16>()?,
                _shnum: self.read_val::<u16>()?,
                _shstrndx: self.read_val::<u16>()?,
            },
        };

        Ok(ehdr)
    }

    /// Parses the program header at `offset`.
    fn parse_phdr(&mut self, offset: u64) -> Result<Phdr, Error> {
        self.data.seek(SeekFrom::Start(offset))?;

        let phdr = match self.size {
            Size::Bits32 => Phdr {
                ptype: self.read_val::<u32>()?,
                offset: self.read_val::<u32>()? as u64,
                vaddr: self.read_val::<u32>()? as u64,
                _paddr: self.read_val::<u32>()? as u64,
                filesz: self.read_val::<u32>()? as u64,
                memsz: self.read_val::<u32>()? as u64,
                _flags: self.read_val::<u32>()?,
                _align: self.read_val::<u32>()? as u64,
            },
            Size::Bits64 => Phdr {
                ptype: self.read_val::<u32>()?,
                _flags: self.read_val::<u32>()?,
                offset: self.read_val::<u64>()?,
                vaddr: self.read_val::<u64>()?,
                _paddr: self.read_val::<u64>()?,
                filesz: self.read_val::<u64>()?,
                memsz: self.read_val::<u64>()?,
                _align: self.read_val::<u64>()?,
            },
        };

        Ok(phdr)
    }

    /// Creates a value from its representation as a byte array, respecting the
    /// endianness of the ELF object being parsed.
    fn read_val<T: FromBytes>(&mut self) -> Result<T, Error> {
        let val = match self.endian {
            Endian::Little => self.data.read_le::<T>()?,
            Endian::Big => self.data.read_be::<T>()?,
        };

        Ok(val)
    }
}
