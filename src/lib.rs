//! This crate allows to generate a flat binary with the memory representation
//! of an ELF. It also allows to generate a FLATELF with the following format:
//!
//! ```text
//! ["FLATELF1"][entry][base_vaddr][flatbin_size][flatbin]
//! ```

mod elf;
mod endian_read;
pub mod error;

use error::Error;

/// Represents a FLATELF.
pub struct FlatElf {
    /// Entry point of the flat binary.
    entry: u64,

    /// Base virtual address of the flat binary.
    base_vaddr: u64,

    /// Flat binary contents.
    flatbin: Vec<u8>,
}

impl FlatElf {
    /// Returns a new `FlatElf`. `data` must be a valid ELF file.
    pub fn new<B: AsRef<[u8]>>(data: &B) -> Result<FlatElf, Error> {
        // Get LOAD program headers.
        let elf = elf::Elf::new(&data)?;
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
        let base_vaddr =
            load_phdrs.first().ok_or(Error::NoLoadSegments)?.vaddr();

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
            let src = data
                .as_ref()
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

        Ok(FlatElf {
            entry: elf.entry(),
            base_vaddr,
            flatbin,
        })
    }

    /// Returns a FLATELF.
    pub fn flatelf(&self) -> Result<Vec<u8>, Error> {
        let mut flatelf = Vec::new();
        flatelf.extend(b"FLATELF1");
        flatelf.extend(self.entry.to_le_bytes());
        flatelf.extend(self.base_vaddr.to_le_bytes());
        flatelf.extend(u64::try_from(self.flatbin.len())?.to_le_bytes());
        flatelf.extend(&self.flatbin);
        Ok(flatelf)
    }

    /// Returns the entry point of the flat binary.
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// Returns the base virtual address of the flat binary.
    pub fn base_vaddr(&self) -> u64 {
        self.base_vaddr
    }

    /// Returns a flat binary.
    pub fn flatbin(&self) -> Vec<u8> {
        self.flatbin.clone()
    }
}
