//! # Executable and Linking Format (ELF)
//!
//! See the [ELF Specification] for more information.
//!
//! [ELF Specification]: https://gabi.xinuos.com/v42/index.html

use core::fmt;



#[derive(Clone, Copy, Debug)]
pub struct ElfFile<'a> {
    pub input: &'a [u8],
    pub header: Header<'a>,
}

impl<'a> ElfFile<'a> {
    pub fn new(input: &'a [u8]) -> Result<ElfFile<'a>, &'static str> {
        let header = Header::parse(input)?;

        Ok(Self { input, header })
    }

    pub fn get_section_header(&self, index: u16) -> Result<&'a SectionHeader, &'static str> {
        SectionHeader::parse(self.input, self.header, index)
    }

    pub fn get_shstr_table(&self) -> Result<&'a [u8], &'static str> {
        let header = self.get_section_header(self.header.body.sh_str_index);
        header.and_then(|h| {
            let offset = h.offset as usize;
            if self.input.len() < offset {
                return Err("File is shorter than section offset");
            }
            Ok(&self.input[offset..])
        })
    }

    pub fn get_shstr(&self, index: u32) -> Result<&'a str, &'static str> {
        self.get_shstr_table().map(|shstr_table| unsafe {
            str::from_utf8_unchecked(read_until_null(&shstr_table[(index as usize)..]))
        })
    }
}



#[derive(Clone, Copy, Debug)]
pub struct Header<'a> {
    pub ident: &'a HeaderIdent,
    pub body: &'a HeaderBody,
}

const MAGIC_NUM: [u8; 4] = [0x7f, b'E', b'L', b'F'];

const HEADER_IDENT_SIZE: usize = size_of::<HeaderIdent>();
const HEADER_BODY_SIZE: usize = size_of::<HeaderBody>();

impl<'a> Header<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, &'static str> {
        if bytes.len() < HEADER_IDENT_SIZE {
            return Err("File is shorter than ELF ident");
        }
        if bytes.len() < HEADER_IDENT_SIZE + HEADER_BODY_SIZE {
            return Err("File is shorter than ELF header");
        }

        let ident: &'a HeaderIdent = unsafe { pod_read(&bytes[..HEADER_IDENT_SIZE]) };

        if ident.magic != MAGIC_NUM {
            return Err("Invalid magic number");
        }
        if ident.class != 2 {
            return Err("Invalid class");
        }

        let body: &'a HeaderBody =
            unsafe { pod_read(&bytes[HEADER_IDENT_SIZE..HEADER_IDENT_SIZE + HEADER_BODY_SIZE]) };

        Ok(Header { ident, body })
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeaderIdent {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub os_abi: u8,
    pub abi_version: u8,
    pub padding: [u8; 7],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeaderBody {
    pub type_: u16,
    pub machine: u16,
    pub version: u32,
    pub entry_point: u64,
    pub ph_offset: u64,
    pub sh_offset: u64,
    pub flags: u32,
    pub header_size: u16,
    pub ph_entry_size: u16,
    pub ph_count: u16,
    pub sh_entry_size: u16,
    pub sh_count: u16,
    pub sh_str_index: u16,
}

impl HeaderIdent {
    pub const fn is_little_endian(&self) -> bool {
        self.data == 1
    }
}

impl<'a> fmt::Display for Header<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "ELF header:")?;
        write!(f, "{}", self.ident)?;
        write!(f, "{}", self.body)?;
        Ok(())
    }
}

impl fmt::Display for HeaderIdent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    magic:            {:?}", self.magic)?;
        writeln!(f, "    class:            {:?}", self.class)?;
        writeln!(f, "    data:             {:?}", self.data)?;
        writeln!(f, "    version:          {:?}", self.version)?;
        writeln!(f, "    os abi:           {:?}", self.os_abi)?;
        writeln!(f, "    abi version:      {:?}", self.abi_version)?;
        writeln!(f, "    padding:          {:?}", self.padding)?;
        Ok(())
    }
}

impl fmt::Display for HeaderBody {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    type:             {:?}", self.type_)?;
        writeln!(f, "    machine:          {:?}", self.machine)?;
        writeln!(f, "    version:          {}", self.version)?;
        writeln!(f, "    entry_point:      {}", self.entry_point)?;
        writeln!(f, "    ph_offset:        {}", self.ph_offset)?;
        writeln!(f, "    sh_offset:        {}", self.sh_offset)?;
        writeln!(f, "    flags:            {}", self.flags)?;
        writeln!(f, "    header_size:      {}", self.header_size)?;
        writeln!(f, "    ph_entry_size:    {}", self.ph_entry_size)?;
        writeln!(f, "    ph_count:         {}", self.ph_count)?;
        writeln!(f, "    sh_entry_size:    {}", self.sh_entry_size)?;
        writeln!(f, "    sh_count:         {}", self.sh_count)?;
        writeln!(f, "    sh_str_index:     {}", self.sh_str_index)?;
        Ok(())
    }
}



pub const SHN_UNDEF: u16 = 0;
pub const SHN_LORESERVE: u16 = 0xff00;
pub const SHN_LOPROC: u16 = 0xff00;
pub const SHN_HIPROC: u16 = 0xff1f;
pub const SHN_LOOS: u16 = 0xff20;
pub const SHN_HIOS: u16 = 0xff3f;
pub const SHN_ABS: u16 = 0xfff1;
pub const SHN_COMMON: u16 = 0xfff2;
pub const SHN_XINDEX: u16 = 0xffff;
pub const SHN_HIRESERVE: u16 = 0xffff;

#[repr(C)]
pub struct SectionHeader {
    name: u32,
    type_: u32,
    flags: u64,
    address: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entry_size: u64,
}

impl SectionHeader {
    pub fn parse<'a>(
        input: &'a [u8],
        header: Header<'a>,
        index: u16,
    ) -> Result<&'a Self, &'static str> {
        assert!(
            index < SHN_LORESERVE,
            "Attempted to get section for a reserved index"
        );

        let start = (index as u64 * header.body.sh_entry_size as u64 + header.body.sh_offset as u64)
            as usize;
        let end = start + header.body.sh_entry_size as usize;

        if input.len() < end {
            return Err("File is shorter than section header offset");
        }

        Ok(unsafe { pod_read(&input[start..end]) })
    }

    pub fn get_name<'a>(&self, file: &ElfFile<'a>) -> Result<&'a str, &'static str> {
        if self.type_ == 0 {
            return Err("Attempted to get name of null section");
        }
        file.get_shstr(self.name)
    }
}

pub struct SectionIter<'input, 'file> {
    pub file: &'file ElfFile<'input>,
    pub next_index: u16,
}

impl<'input, 'file> Iterator for SectionIter<'input, 'file> {
    type Item = &'input SectionHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.file.header.body.sh_count {
            return None;
        }

        let result = self.file.get_section_header(self.next_index);
        self.next_index += 1;

        result.ok()
    }
}




pub unsafe trait Pod: Sized {}

unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for u64 {}
unsafe impl Pod for u128 {}

unsafe impl Pod for i8 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for i32 {}
unsafe impl Pod for i64 {}
unsafe impl Pod for i128 {}

unsafe impl Pod for HeaderIdent {}
unsafe impl Pod for HeaderBody {}
unsafe impl Pod for SectionHeader {}

unsafe fn pod_read<T: Pod>(bytes: &[u8]) -> &T {
    assert!(size_of::<T>() <= bytes.len());
    let addr = bytes.as_ptr() as usize;
    // Alignment is always a power of 2, so we can use bit ops instead of a mod
    // here.
    assert!((addr & (align_of::<T>() - 1)) == 0);


    unsafe { &*(bytes.as_ptr() as *const T) }
}

fn read_until_null(input: &[u8]) -> &[u8] {
    for (i, byte) in input.iter().enumerate() {
        if *byte == 0 {
            return &input[..i];
        }
    }

    panic!("No null byte in input");
}
