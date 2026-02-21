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

    pub fn section_iter(&self) -> impl Iterator<Item = &SectionHeader> + '_ {
        SectionIter {
            file: self,
            next_index: 0,
        }
    }

    pub fn find_section_by_name(&self, name: &str) -> Option<&SectionHeader> {
        for sect in self.section_iter() {
            if let Ok(sect_name) = sect.get_name(self) {
                if sect_name == name {
                    return Some(sect);
                }
            }
        }

        None
    }

    pub fn get_string(&self, index: u32) -> Result<&'a str, &'static str> {
        let header = self
            .find_section_by_name(".strtab")
            .ok_or("no `.strtab` section")?;
        if header.get_type()? != SectionHeaderType::StrTab {
            return Err("expected `.strtab` to be a string table");
        }
        Ok(read_str(&header.raw_data(self)[(index as usize)..]))
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

    pub fn get_symbol_table(&self) -> Result<&[SymbolTableEntry], &'static str> {
        let symtab_data = self
            .section_iter()
            .find(|sec| sec.get_type() == Ok(SectionHeaderType::SymTab))
            .ok_or("no `symtab` section")
            .and_then(|s| s.get_data(self));

        match symtab_data {
            Ok(SectionData::SymbolTable(symtab)) => Ok(symtab),
            _ => Err("no symbol table found, file may have been stripped"),
        }
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

    pub fn get_type(&self) -> ObjectFileType {
        match self.body.type_ {
            0 => ObjectFileType::None,
            1 => ObjectFileType::Relocatable,
            2 => ObjectFileType::Executable,
            3 => ObjectFileType::SharedObject,
            4 => ObjectFileType::Core,
            x => ObjectFileType::ProcessorSpecific(x),
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectFileType {
    None,
    Relocatable,
    Executable,
    SharedObject,
    Core,
    ProcessorSpecific(u16),
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
        if self.get_type()? == SectionHeaderType::Null {
            return Err("Attempted to get name of null section");
        }
        file.get_shstr(self.name)
    }

    pub fn get_type(&self) -> Result<SectionHeaderType, &'static str> {
        match self.type_ {
            0 => Ok(SectionHeaderType::Null),
            1 => Ok(SectionHeaderType::ProgBits),
            2 => Ok(SectionHeaderType::SymTab),
            3 => Ok(SectionHeaderType::StrTab),
            4 => Ok(SectionHeaderType::Rela),
            5 => Ok(SectionHeaderType::Hash),
            6 => Ok(SectionHeaderType::Dynamic),
            7 => Ok(SectionHeaderType::Note),
            8 => Ok(SectionHeaderType::NoBits),
            9 => Ok(SectionHeaderType::Rel),
            10 => Ok(SectionHeaderType::ShLib),
            11 => Ok(SectionHeaderType::DynSym),
            // sic.
            14 => Ok(SectionHeaderType::InitArray),
            15 => Ok(SectionHeaderType::FiniArray),
            16 => Ok(SectionHeaderType::PreInitArray),
            17 => Ok(SectionHeaderType::Group),
            18 => Ok(SectionHeaderType::SymTabShIndex),
            n if (SHT_LOOS..=SHT_HIOS).contains(&n) => Ok(SectionHeaderType::OsSpecific(n)),
            n if (SHT_LOPROC..=SHT_HIPROC).contains(&n) => {
                Ok(SectionHeaderType::ProcessorSpecific(n))
            }
            n if (SHT_LOUSER..=SHT_HIUSER).contains(&n) => Ok(SectionHeaderType::User(n)),
            _ => Err("Invalid section header type"),
        }
    }

    pub fn get_data<'a>(&self, file: &ElfFile<'a>) -> Result<SectionData<'a>, &'static str> {
        self.get_type().and_then(|typ| {
            Ok(match typ {
                SectionHeaderType::Null | SectionHeaderType::NoBits => SectionData::Empty,
                SectionHeaderType::ProgBits
                | SectionHeaderType::ShLib
                | SectionHeaderType::OsSpecific(_)
                | SectionHeaderType::ProcessorSpecific(_)
                | SectionHeaderType::User(_) => SectionData::Undefined(self.raw_data(file)),
                SectionHeaderType::SymTab => {
                    let data = self.raw_data(file);
                    SectionData::SymbolTable(read_array(data))
                }
                SectionHeaderType::DynSym => {
                    let data = self.raw_data(file);
                    SectionData::DynSymbolTable(read_array(data))
                }
                SectionHeaderType::StrTab => SectionData::StrArray(self.raw_data(file)),
                SectionHeaderType::InitArray
                | SectionHeaderType::FiniArray
                | SectionHeaderType::PreInitArray => {
                    let data = self.raw_data(file);
                    SectionData::FnArray(read_array(data))
                }
                SectionHeaderType::Rela => {
                    let data = self.raw_data(file);
                    SectionData::Rela(read_array(data))
                }
                SectionHeaderType::Rel => {
                    let data = self.raw_data(file);
                    SectionData::Rel(read_array(data))
                }
                SectionHeaderType::Dynamic => {
                    todo!()
                    // let data = self.raw_data(file);
                    // SectionData::Dynamic(read_array(data))
                }
                SectionHeaderType::Group => {
                    let data = self.raw_data(file);
                    unsafe {
                        let flags: &'a u32 = std::mem::transmute(&data[0]);
                        let indices: &'a [u32] = read_array(&data[4..]);
                        SectionData::Group { flags, indices }
                    }
                }
                SectionHeaderType::SymTabShIndex => {
                    SectionData::SymTabShIndex(read_array(self.raw_data(file)))
                }
                SectionHeaderType::Note => todo!(),
                SectionHeaderType::Hash => todo!(),
            })
        })
    }

    pub fn raw_data<'a>(&self, file: &ElfFile<'a>) -> &'a [u8] {
        assert_ne!(self.get_type().unwrap(), SectionHeaderType::Null);
        &file.input[self.offset() as usize..(self.offset() + self.size()) as usize]
    }

    #[inline]
    pub const fn address(&self) -> u64 {
        self.address
    }

    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[inline]
    pub const fn align(&self) -> u64 {
        self.align
    }

    #[inline]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    #[inline]
    pub const fn flags(&self) -> u64 {
        self.flags
    }

    #[inline]
    pub const fn name(&self) -> u32 {
        self.name
    }

    #[inline]
    pub const fn link(&self) -> u32 {
        self.link
    }

    #[inline]
    pub const fn info(&self) -> u32 {
        self.info
    }
}

pub const SHF_WRITE: u64 = 0x1;
pub const SHF_ALLOC: u64 = 0x2;
pub const SHF_EXECINSTR: u64 = 0x4;
pub const SHF_MERGE: u64 = 0x10;
pub const SHF_STRINGS: u64 = 0x20;
pub const SHF_INFO_LINK: u64 = 0x40;
pub const SHF_LINK_ORDER: u64 = 0x80;
pub const SHF_OS_NONCONFORMING: u64 = 0x100;
pub const SHF_GROUP: u64 = 0x200;
pub const SHF_TLS: u64 = 0x400;
pub const SHF_COMPRESSED: u64 = 0x800;
pub const SHF_MASKOS: u64 = 0x0ff00000;
pub const SHF_MASKPROC: u64 = 0xf0000000;

pub const SHT_LOOS: u32 = 0x60000000;
pub const SHT_HIOS: u32 = 0x6fffffff;
pub const SHT_LOPROC: u32 = 0x70000000;
pub const SHT_HIPROC: u32 = 0x7fffffff;
pub const SHT_LOUSER: u32 = 0x80000000;
pub const SHT_HIUSER: u32 = 0xffffffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionHeaderType {
    Null,
    ProgBits,
    SymTab,
    StrTab,
    Rela,
    Hash,
    Dynamic,
    Note,
    NoBits,
    Rel,
    ShLib,
    DynSym,
    InitArray,
    FiniArray,
    PreInitArray,
    Group,
    SymTabShIndex,
    OsSpecific(u32),
    ProcessorSpecific(u32),
    User(u32),
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

#[derive(Debug)]
pub enum SectionData<'a> {
    Empty,
    Undefined(&'a [u8]),
    Group { flags: &'a u32, indices: &'a [u32] },
    StrArray(&'a [u8]),
    FnArray(&'a [u64]),
    SymbolTable(&'a [SymbolTableEntry]),
    DynSymbolTable(&'a [SymbolTableEntry]),
    SymTabShIndex(&'a [u32]),
    Rela(&'a [Rela]),
    Rel(&'a [Rel]),
    // Dynamic(&'a [Dynamic]),
}

#[derive(Debug)]
#[repr(C)]
pub struct Rel {
    offset: u64,
    info: u64,
}
#[derive(Debug)]
#[repr(C)]
pub struct Rela {
    offset: u64,
    info: u64,
    addend: u64,
}

impl Rela {
    pub fn get_offset(&self) -> u64 {
        self.offset
    }

    pub fn get_addend(&self) -> u64 {
        self.addend
    }

    pub fn get_symbol_table_index(&self) -> u32 {
        (self.info >> 32) as u32
    }

    pub fn get_type(&self) -> u32 {
        (self.info & 0xffffffff) as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolType {
    NoType,
    Object,
    Func,
    Section,
    File,
    Common,
    Tls,
    OsSpecific(u8),
    ProcessorSpecific(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
    OsSpecific(u8),
    ProcessorSpecific(u8),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum SymbolVisibility {
    #[default]
    Default = 0,
    Internal = 1,
    Hidden = 2,
    Protected = 3,
}

#[derive(Debug)]
#[repr(C)]
pub struct SymbolTableEntry {
    name: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

impl SymbolTableEntry {
    /// Get the type (function, TLS, file, etc.) of this symbol.
    pub fn get_type(&self) -> Result<SymbolType, &'static str> {
        match self.info() & 0xf {
            0 => Ok(SymbolType::NoType),
            1 => Ok(SymbolType::Object),
            2 => Ok(SymbolType::Func),
            3 => Ok(SymbolType::Section),
            4 => Ok(SymbolType::File),
            5 => Ok(SymbolType::Common),
            6 => Ok(SymbolType::Tls),
            b @ 10..=12 => Ok(SymbolType::OsSpecific(b)),
            b @ 13..=15 => Ok(SymbolType::ProcessorSpecific(b)),

            _ => Err("invalid value for symbol type"),
        }
    }

    /// Get the binding (local, global, weak, etc.) of this symbol.
    pub fn get_binding(&self) -> Result<SymbolBinding, &'static str> {
        match self.info() >> 4 {
            0 => Ok(SymbolBinding::Local),
            1 => Ok(SymbolBinding::Global),
            2 => Ok(SymbolBinding::Weak),
            b if (10..=12).contains(&b) => Ok(SymbolBinding::OsSpecific(b)),
            b if (13..=15).contains(&b) => Ok(SymbolBinding::ProcessorSpecific(b)),

            _ => Err("invalid value for symbol binding"),
        }
    }

    /// Get the visibility (default, internal, hidden, or protected) of this
    /// symbol.
    ///
    /// This cannot fail because there are no invalid values for symbol
    /// visibility.
    pub fn get_visibility(&self) -> SymbolVisibility {
        match self.other & 0x3 {
            x if x == SymbolVisibility::Default as _ => SymbolVisibility::Default,
            x if x == SymbolVisibility::Internal as _ => SymbolVisibility::Internal,
            x if x == SymbolVisibility::Hidden as _ => SymbolVisibility::Hidden,
            x if x == SymbolVisibility::Protected as _ => SymbolVisibility::Protected,

            // Covers all possible cases.
            _ => unreachable!(),
        }
    }

    pub fn get_name<'a>(&'a self, file: &ElfFile<'a>) -> Result<&'a str, &'static str> {
        file.get_string(self.name)
    }

    pub fn get_section_header<'a>(
        &'a self,
        file: &ElfFile<'a>,
        self_index: usize,
    ) -> Result<&'a SectionHeader, &'static str> {
        match self.shndx {
            SHN_XINDEX => {
                let header = file.find_section_by_name(".symtab_shndx");
                if let Some(header) = header {
                    assert_eq!(header.get_type()?, SectionHeaderType::SymTabShIndex);
                    if let SectionData::SymTabShIndex(data) = header.get_data(file)? {
                        let index = data[self_index] as u16;
                        assert_ne!(index, SHN_UNDEF);
                        file.get_section_header(index)
                    } else {
                        Err("expected SymTabShIndex")
                    }
                } else {
                    Err("no `.symtab_shndx` section")
                }
            }
            SHN_UNDEF | SHN_ABS | SHN_COMMON => Err("reserved section header index"),
            i => file.get_section_header(i),
        }
    }

    #[inline]
    pub const fn name(&self) -> u32 {
        self.name
    }

    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[inline]
    pub const fn value(&self) -> u64 {
        self.value
    }

    #[inline]
    pub const fn info(&self) -> u8 {
        self.info
    }

    #[inline]
    pub const fn shndx(&self) -> u16 {
        self.shndx
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
unsafe impl Pod for Rel {}
unsafe impl Pod for Rela {}
unsafe impl Pod for SymbolTableEntry {}

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

fn read_str(input: &[u8]) -> &str {
    std::str::from_utf8(read_str_bytes(input)).expect("invalid UTF-8 string")
}

fn read_str_bytes(input: &[u8]) -> &[u8] {
    for (i, byte) in input.iter().enumerate() {
        if *byte == 0 {
            return &input[..i];
        }
    }

    panic!("no null byte in input");
}

fn read_array<T: Pod>(input: &[u8]) -> &[T] {
    let t_size = size_of::<T>();
    assert!(t_size > 0, "Can't read arrays of zero-sized types");
    assert!(input.len() % t_size == 0);
    let addr = input.as_ptr() as usize;
    assert!(addr & (align_of::<T>() - 1) == 0);

    unsafe { read_array_unsafe(input) }
}

unsafe fn read_array_unsafe<T: Sized>(input: &[u8]) -> &[T] {
    let ptr = input.as_ptr() as *const T;
    unsafe { std::slice::from_raw_parts(ptr, input.len() / size_of::<T>()) }
}
