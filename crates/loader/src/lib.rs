//! # Loader
//!
//! The runtime [linker]/[loader].
//!
//! [linker]: https://en.wikipedia.org/wiki/Linker_(computing)
//! [loader]: https://en.wikipedia.org/wiki/Loader_(computing)

// #![no_std]
#![feature(fn_ptr_trait)]

// #[macro_use]
// extern crate alloc;

use std::{
    collections::{BTreeSet, HashMap},
    marker::FnPtr,
    ops::Range,
    sync::{Arc, Weak},
};

#[cfg(target_arch = "x86_64")]
use abi::elf::Rela;
use {
    abi::{
        elf::{
            ElfFile, ObjectFileType, SHF_ALLOC, SHF_EXECINSTR, SHF_TLS, SHF_WRITE, SectionData,
            SectionHeaderType, SymbolBinding, SymbolType,
        },
        mem::{MapFlags, MemoryMap},
    },
    spin::Mutex,
};



/// A set of loaded [objects](LoadedObject) and [sections](LoadedSection).
#[derive(Debug)]
pub struct Loader {
    search_path: String,
    objects: Mutex<HashMap<Arc<str>, Arc<Mutex<LoadedObject>>>>,
    sections: Mutex<HashMap<Arc<str>, Weak<LoadedSection>>>,
}

/// An object that has been loaded into memory.
#[derive(Debug)]
pub struct LoadedObject {
    /// The demangled name of this object.
    pub name: Arc<str>,
    /// The sections that have been loaded into memory for this object.
    pub sections: HashMap<usize, Arc<LoadedSection>>,
    /// A set of section indices representing the global sections of this
    /// object. They can be used as keys for [`self.sections`](Self::sections).
    pub global_sections: BTreeSet<usize>,
    /// A set of section indices representing the data sections of this object.
    /// They can be used as keys for [`self.sections`](Self::sections).
    pub data_sections: BTreeSet<usize>,
    /// A set of section indices representing the thread-local storage (TLS)
    /// sections of this object. They can be used as keys for
    /// [`self.sections`](Self::sections).
    pub tls_sections: BTreeSet<usize>,
}

/// An object section that has been loaded into memory.
#[derive(Debug)]
pub struct LoadedSection {
    /// The demangled name of this section.
    pub name: Arc<str>,
    /// The type of this section (`.text`, `.data`, etc.).
    pub kind: SectionKind,
    /// Whether this section is global (public).
    pub global: bool,
    /// The size of this section in bytes.
    pub size: usize,
    /// The memory address of this section.
    pub addr: usize,
    /// A reference to the mapping that contains this section's data.
    pub mapping: Arc<Mutex<MemoryMap>>,
    /// The offset into [`self.mapping`](Self::mapping) at which this section's
    /// data starts.
    pub mapping_offset: usize,
    /// The object that contains this section.
    pub owner: Weak<Mutex<LoadedObject>>,
}



impl Loader {
    pub fn new(search_path: &str) -> Self {
        Self {
            search_path: search_path.into(),
            objects: Mutex::new(HashMap::new()),
            sections: Mutex::new(HashMap::new()),
        }
    }

    // FIXME: This shouldn't be fallible.
    pub fn find_object_files(&self, prefix: &str) -> Result<Vec<String>, &'static str> {
        let mut paths = Vec::new();
        for entry in
            std::fs::read_dir(&self.search_path).map_err(|_| "failed to read search directory")?
        {
            let Ok(entry) = entry else {
                continue;
            };
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "found invalid file name in search directory")?;
            if name.starts_with(prefix) && name.ends_with(".o") {
                paths.push(name);
            }
        }

        Ok(paths)
    }

    pub fn get_object(&self, name: &str) -> Option<Weak<Mutex<LoadedObject>>> {
        self.objects.lock().get(name).map(Arc::downgrade)
    }

    pub fn get_section(&self, name: &str) -> Option<Weak<LoadedSection>> {
        self.sections.lock().get(name).cloned()
    }

    pub fn get_section_ending_with(&self, postfix: &str) -> Option<Weak<LoadedSection>> {
        self.sections
            .lock()
            .iter()
            .find(|(name, _section)| name.ends_with(postfix))
            .map(|(_name, section)| section.clone())
    }

    pub fn get_or_load_section(&self, name: &str) -> Weak<LoadedSection> {
        if let Some(section) = self.sections.lock().get(name) {
            return section.clone();
        }

        for crate_name in crate_names_in_symbol(name) {
            println!("SYM @ `{name}` = '{crate_name}'");
            for object_file_name in self.find_object_files(crate_name).unwrap() {
                let object_name = object_file_name
                    .strip_suffix(".o")
                    .expect("Loader::find_object_files should only return names ending with '.o'");
                // Skip already loaded objects.
                if self.get_object(object_name).is_some() {
                    continue;
                }
                println!("LOADING OBJECT '{object_name}' @ `{name}`");
                self.load_object(
                    object_name,
                    &std::fs::read(format!("{}/{object_file_name}", self.search_path)).unwrap(),
                )
                .unwrap();
                if let Some(section) = self.sections.lock().get(name) {
                    return section.clone();
                }
            }
        }

        panic!("failed to load `{name}`")
    }

    pub fn load_object(
        &self,
        object_name: &str,
        object_bytes: &[u8],
    ) -> Result<Arc<Mutex<LoadedObject>>, &'static str> {
        let (object, elf_file) = self.load_object_sections(object_name, object_bytes)?;
        self.add_sections(object.lock().sections.values());
        self.objects
            .lock()
            .insert(object_name.into(), Arc::clone(&object));
        self.relocate_object_sections(&elf_file, &object)?;

        Ok(object)
    }

    fn load_object_sections<'obj>(
        &self,
        object_name: &'obj str,
        object_bytes: &'obj [u8],
    ) -> Result<(Arc<Mutex<LoadedObject>>, ElfFile<'obj>), &'static str> {
        let elf_file = ElfFile::new(object_bytes)?;
        if elf_file.header.get_type() != ObjectFileType::Relocatable {
            return Err("not a relocatable ELF file");
        }

        let SectionMappings {
            executable: executable_mapping,
            read_only: read_only_mapping,
            read_write: read_write_mapping,
        } = allocate_section_mappings(&elf_file)?;

        let executable_mapping = Arc::new(Mutex::new(executable_mapping));
        let read_only_mapping = Arc::new(Mutex::new(read_only_mapping));
        let read_write_mapping = Arc::new(Mutex::new(read_write_mapping));

        // The `.text` sections always come at the beginning, so we can get the byte
        // range without needing to know the offset.
        {
            let mut executable_map_lock = executable_mapping.lock();
            let text_size = executable_map_lock.len();
            let slice = elf_file.input.get(..text_size).ok_or(
                "end of last `.text` section was miscalculated to be beyond ELF file bounds",
            )?;
            executable_map_lock.copy_from_slice(slice);
        }

        let mut read_only_map_lock = read_only_mapping.lock();
        let mut read_write_map_lock = read_write_mapping.lock();

        let object = Arc::new(Mutex::new(LoadedObject {
            name: rustc_demangle::demangle(object_name).to_string().into(),
            sections: HashMap::new(),
            global_sections: BTreeSet::new(),
            data_sections: BTreeSet::new(),
            tls_sections: BTreeSet::new(),
        }));

        let mut loaded_sections: HashMap<usize, Arc<LoadedSection>> = HashMap::new();
        let mut data_sections: BTreeSet<usize> = BTreeSet::new();
        let mut tls_sections: BTreeSet<usize> = BTreeSet::new();
        let global_sections: BTreeSet<usize> = {
            let symbol_table = elf_file.get_symbol_table()?;
            let mut globals: BTreeSet<usize> = BTreeSet::new();
            for entry in symbol_table.iter() {
                if entry.get_binding() == Ok(SymbolBinding::Global) {
                    match entry.get_type() {
                        Ok(SymbolType::Func | SymbolType::Object | SymbolType::Tls) => {
                            globals.insert(entry.shndx() as usize);
                        }
                        _ => continue,
                    }
                }
            }

            globals
        };

        let mut rodata_offset = 0;
        let mut data_offset = 0;

        for (section_index, section) in elf_file.section_iter().enumerate() {
            let section_flags = section.flags();

            // Skip non-allocated sections.
            if section_flags & SHF_ALLOC == 0 {
                continue;
            }

            // If the current section is zero-sized, it's a reference to the next section.
            // So, we just use the next section's information (size, align, etc.) with the
            // current section's name.
            let section_name = section.get_name(&elf_file)?;
            let section = if section.size() == 0 {
                // If the next section has the same offset as the current one, use it instead of
                // the current one.
                match elf_file.get_section_header((section_index + 1) as u16) {
                    Ok(next_section) => {
                        if next_section.offset() == section.offset() {
                            next_section
                        } else {
                            section
                        }
                    }
                    _ => {
                        return Err("couldn't get the section following a zero-sized section");
                    }
                }
            } else {
                section
            };

            let section_size = section.size() as usize;
            let section_align = section.align() as usize;

            let is_write = section_flags & SHF_WRITE == SHF_WRITE;
            let is_exec = section_flags & SHF_EXECINSTR == SHF_EXECINSTR;
            let is_tls = section_flags & SHF_TLS == SHF_TLS;

            macro_rules! symbol_name_after_prefix {
                ($sec_name:ident, $prefix:literal) => {
                    if let Some(name) = $sec_name.get($prefix.len()..) {
                        name
                    } else {
                        // Ignore placeholder sections.
                        match $sec_name {
                            ".text" | ".rodata" | ".data" | ".bss" => continue,
                            _ => {
                                return Err(concat!(
                                    "failed to get the ",
                                    $prefix,
                                    " section's name after '",
                                    $prefix,
                                    "'"
                                ));
                            }
                        }
                    }
                };
            }

            // .text
            if is_exec && !is_write {
                let is_global = global_sections.contains(&section_index);
                let name = symbol_name_after_prefix!(section_name, ".text.");
                let name = if is_global && name.starts_with("unlikely.") {
                    name.get("unlikely.".len()..)
                        .ok_or("failed to get `.text.unlikely.` section's name")?
                } else {
                    name
                };

                // We already copied the content of all `.text` sections above, so here we just
                // record the metadata into a new `LoadedSection` object.
                let text_offset = section.offset() as usize;
                let section_addr = executable_mapping.lock().addr() + text_offset;

                loaded_sections.insert(
                    section_index,
                    Arc::new(LoadedSection {
                        name: rustc_demangle::demangle(name).to_string().into(),
                        kind: SectionKind::Text,
                        size: section_size,
                        addr: section_addr,
                        global: is_global,
                        mapping: Arc::clone(&executable_mapping),
                        mapping_offset: text_offset,
                        owner: Arc::downgrade(&object),
                    }),
                );
            }
            // .tdata/.tbss
            else if is_tls {
                // check if this TLS section is .bss or .data
                let is_bss = section.get_type() == Ok(SectionHeaderType::NoBits);
                let name = if is_bss {
                    symbol_name_after_prefix!(section_name, ".tbss.")
                } else {
                    symbol_name_after_prefix!(section_name, ".tdata.")
                };

                let (mapping_offset, kind) = if is_bss {
                    // Offset is irrelevant here.
                    (usize::MAX, SectionKind::TlsBss)
                } else {
                    let slice = read_only_map_lock.as_slice_mut(rodata_offset, section_size);
                    match section.get_data(&elf_file) {
                        Ok(SectionData::Undefined(sec_data)) => slice.copy_from_slice(sec_data),
                        _ => {
                            return Err("couldn't get data for `.tdata` section");
                        }
                    };

                    (rodata_offset, SectionKind::TlsData)
                };

                let tls_section = Arc::new(LoadedSection {
                    name: rustc_demangle::demangle(name).to_string().into(),
                    kind,
                    size: section_size,
                    addr: 0, // See below.
                    global: global_sections.contains(&section_index),
                    mapping: Arc::clone(&read_only_mapping),
                    mapping_offset,
                    owner: Arc::downgrade(&object),
                });

                // This should initialize a TLS area and set the section's address.
                if true {
                    return Err("TODO: TLS section initialization");
                }

                loaded_sections.insert(section_index, tls_section);
                tls_sections.insert(section_index);

                rodata_offset += section_size.next_multiple_of(section_align);
            }
            // .data/.bss
            else if is_write {
                let is_bss = section.get_type() == Ok(SectionHeaderType::NoBits);
                let name = if is_bss {
                    symbol_name_after_prefix!(section_name, ".bss.")
                } else {
                    symbol_name_after_prefix!(section_name, ".data.")
                };

                assert!(data_offset < read_write_map_lock.len());
                let section_addr = read_write_map_lock.addr() + data_offset;

                let slice = read_write_map_lock.as_slice_mut(data_offset, section_size);
                match section.get_data(&elf_file) {
                    Ok(SectionData::Undefined(sec_data)) => slice.copy_from_slice(sec_data),
                    Ok(SectionData::Empty) => slice.fill(0),
                    _ => {
                        return Err("couldn't get data for `.data` section");
                    }
                }

                loaded_sections.insert(
                    section_index,
                    Arc::new(LoadedSection {
                        name: rustc_demangle::demangle(name).to_string().into(),
                        kind: if is_bss {
                            SectionKind::Bss
                        } else {
                            SectionKind::Data
                        },
                        size: section_size,
                        addr: section_addr,
                        global: global_sections.contains(&section_index),
                        mapping: Arc::clone(&read_write_mapping),
                        mapping_offset: data_offset,
                        owner: Arc::downgrade(&object),
                    }),
                );
                data_sections.insert(section_index);

                data_offset += section_size.next_multiple_of(section_align);
            }
            // .rodata
            else if section_name.starts_with(".rodata") {
                let name = symbol_name_after_prefix!(section_name, ".rodata.");

                assert!(rodata_offset < read_only_map_lock.len());
                let section_addr = read_only_map_lock.addr() + rodata_offset;

                let slice = read_only_map_lock.as_slice_mut(rodata_offset, section_size);
                match section.get_data(&elf_file) {
                    Ok(SectionData::Undefined(sec_data)) => slice.copy_from_slice(sec_data),
                    Ok(SectionData::Empty) => slice.fill(0),
                    _ => {
                        return Err("couldn't get data for `.rodata` section");
                    }
                }

                loaded_sections.insert(
                    section_index,
                    Arc::new(LoadedSection {
                        name: rustc_demangle::demangle(name).to_string().into(),
                        kind: SectionKind::Rodata,
                        size: section_size,
                        addr: section_addr,
                        global: global_sections.contains(&section_index),
                        mapping: Arc::clone(&read_only_mapping),
                        mapping_offset: rodata_offset,
                        owner: Arc::downgrade(&object),
                    }),
                );

                rodata_offset += section_size.next_multiple_of(section_align);
            }
            // .gcc_except_table
            else if section_name.starts_with(".gcc_except_table") {
                assert!(rodata_offset < read_only_map_lock.len());
                let section_addr = read_only_map_lock.addr() + rodata_offset;

                let slice = read_only_map_lock.as_slice_mut(rodata_offset, section_size);
                match section.get_data(&elf_file) {
                    Ok(SectionData::Undefined(sec_data)) => slice.copy_from_slice(sec_data),
                    Ok(SectionData::Empty) => slice.fill(0),
                    _ => {
                        return Err("couldn't get data for `.gcc_except_table` section");
                    }
                }

                let kind = SectionKind::GccExceptTable;
                loaded_sections.insert(
                    section_index,
                    Arc::new(LoadedSection {
                        name: kind.name().into(), // Ignore actual table name.
                        kind,
                        size: section_size,
                        addr: section_addr,
                        global: false,
                        mapping: Arc::clone(&read_only_mapping),
                        mapping_offset: rodata_offset,
                        owner: Arc::downgrade(&object),
                    }),
                );

                rodata_offset += section_size.next_multiple_of(section_align);
            }
            // .eh_frame
            else if section_name == ".eh_frame" {
                assert!(rodata_offset < read_only_map_lock.len());
                let section_addr = read_only_map_lock.addr() + rodata_offset;

                let slice = read_only_map_lock.as_slice_mut(rodata_offset, section_size);
                match section.get_data(&elf_file) {
                    Ok(SectionData::Undefined(sec_data)) => slice.copy_from_slice(sec_data),
                    Ok(SectionData::Empty) => slice.fill(0),
                    _ => {
                        return Err("couldn't get data for `.eh_frame` section");
                    }
                }

                let kind = SectionKind::EhFrame;
                loaded_sections.insert(
                    section_index,
                    Arc::new(LoadedSection {
                        name: kind.name().into(), // Ignore actual table name.
                        kind,
                        size: section_size,
                        addr: section_addr,
                        global: false,
                        mapping: Arc::clone(&read_only_mapping),
                        mapping_offset: rodata_offset,
                        owner: Arc::downgrade(&object),
                    }),
                );

                rodata_offset += section_size.next_multiple_of(section_align);
            }
            // Unhandled section.
            else {
                return Err("encountered unhandled section");
            }
        }

        {
            let mut object_lock = object.lock();
            object_lock.sections = loaded_sections;
            object_lock.global_sections = global_sections;
            object_lock.data_sections = data_sections;
            object_lock.tls_sections = tls_sections;
        }

        Ok((object, elf_file))
    }

    fn relocate_object_sections(
        &self,
        elf_file: &ElfFile,
        object: &Arc<Mutex<LoadedObject>>,
    ) -> Result<(), &'static str> {
        let object = object.lock();
        let symbol_table = elf_file.get_symbol_table()?;

        for section in elf_file.section_iter().filter(|section| {
            section.get_type() == Ok(SectionHeaderType::Rela) && section.size() != 0
        }) {
            let rela_array = match section.get_data(elf_file) {
                Ok(SectionData::Rela(rela_arr)) => rela_arr,
                _ => {
                    return Err("found `rela` section that wasn't able to be parsed");
                }
            };

            let target_section_index = section.info() as usize;
            let target_section = object
                .sections
                .get(&target_section_index)
                .ok_or("target section was not loaded for `rela` section")?;

            {
                let mut target_section_mapping = target_section.mapping.lock();
                let target_slice = target_section_mapping
                    .as_slice_mut(0, target_section.mapping_offset + target_section.size);

                for rela_entry in rela_array {
                    let source_entry = &symbol_table[rela_entry.get_symbol_table_index() as usize];
                    let source_index = source_entry.shndx() as usize;
                    let source_value = source_entry.value() as usize;

                    let source_section = match object.sections.get(&source_index) {
                        Some(section) => Ok(section.clone()),
                        None => {
                            let name = source_entry
                                .get_name(&elf_file)
                                .map_err(|_| "couldn't get name of source section")?;
                            let name = if name.starts_with(".data.rel.ro.") {
                                name.get(".data.rel.ro.".len()..).ok_or(
                                    "couldn't get name of `.data.rel.ro.`
                                section",
                                )?
                            } else {
                                name
                            };

                            let demangled_name = rustc_demangle::demangle(name).to_string();

                            self.get_or_load_section(&demangled_name)
                                .upgrade()
                                .ok_or("couldn't get section for relocation entry")
                        }
                    }?;

                    let target_offset =
                        target_section.mapping_offset + rela_entry.get_offset() as usize;

                    write_relocation(
                        rela_entry,
                        target_slice,
                        target_offset,
                        source_section.addr + source_value,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn add_sections<'a, I>(&self, sections: I) -> usize
    where
        I: IntoIterator<Item = &'a Arc<LoadedSection>>,
    {
        let mut map = self.sections.lock();
        let mut added_count = 0;
        for section in sections.into_iter() {
            if section.global {
                let added = map
                    .insert(section.name.clone(), Arc::downgrade(section))
                    .is_none();
                if added {
                    added_count += 1;
                }
            }
        }

        added_count
    }
}

#[cfg(target_arch = "x86_64")]
fn write_relocation(
    relocation_entry: &Rela,
    target_slice: &mut [u8],
    target_offset: usize,
    source_addr: usize,
) -> Result<(), &'static str> {
    // https://docs.rs/goblin/latest/src/goblin/elf/constants_relocation.rs.html
    const R_X86_64_64: u32 = 1;
    const R_X86_64_PC32: u32 = 2;
    const R_X86_64_PLT32: u32 = 4;
    const R_X86_64_32: u32 = 10;
    const R_X86_64_PC64: u32 = 24;

    let source_addr = source_addr as u64;
    match relocation_entry.get_type() {
        R_X86_64_32 => {
            let target_range = target_offset..(target_offset + size_of::<u32>());
            let target_ref = &mut target_slice[target_range];
            let source_value = source_addr.wrapping_add(relocation_entry.get_addend()) as u32;

            target_ref.copy_from_slice(&source_value.to_ne_bytes());
        }
        R_X86_64_PC32 | R_X86_64_PLT32 => {
            let target_range = target_offset..(target_offset + size_of::<u32>());
            let target_ref = &mut target_slice[target_range];
            let source_value = source_addr
                .wrapping_add(relocation_entry.get_addend())
                .wrapping_sub(target_ref.as_ptr() as usize as u64)
                as u32;

            target_ref.copy_from_slice(&source_value.to_ne_bytes());
        }
        R_X86_64_64 => {
            let target_range = target_offset..(target_offset + size_of::<u64>());
            let target_ref = &mut target_slice[target_range];
            let source_value = source_addr.wrapping_add(relocation_entry.get_addend());

            target_ref.copy_from_slice(&source_value.to_ne_bytes());
        }
        R_X86_64_PC64 => {
            let target_range = target_offset..(target_offset + size_of::<u64>());
            let target_ref = &mut target_slice[target_range];
            let source_val = source_addr
                .wrapping_add(relocation_entry.get_addend())
                .wrapping_sub(target_ref.as_ptr() as usize as u64);

            target_ref.copy_from_slice(&source_val.to_ne_bytes());
        }

        _ => return Err("unsupported relocation type"),
    }

    Ok(())
}

impl LoadedSection {
    pub unsafe fn as_function<F: FnPtr>(&self) -> Result<&F, &'static str> {
        if self.kind != SectionKind::Text {
            return Err("tried to interpret non-text section as function");
        }

        let map = self.mapping.lock();

        let end = self.mapping_offset + self.size;
        if end > map.len() {
            return Err("function section is too large for its mapping, this is a logic error");
        }

        // SAFETY: It's up to the caller to make sure the type signature matches.
        Ok(unsafe { core::mem::transmute(&(map.addr() + self.mapping_offset)) })
    }
}



#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionKind {
    Text,
    Rodata,
    Data,
    Bss,
    TlsData,
    TlsBss,
    GccExceptTable,
    EhFrame,
}

impl SectionKind {
    pub fn name(&self) -> &'static str {
        match self {
            SectionKind::Text => ".text",
            SectionKind::Rodata => ".rodata",
            SectionKind::Data => ".data",
            SectionKind::Bss => ".bss",
            SectionKind::TlsData => ".tdata",
            SectionKind::TlsBss => ".tbss",
            SectionKind::GccExceptTable => ".gcc_except_table",
            SectionKind::EhFrame => ".eh_frame",
        }
    }
}



// TODO: This needs to be thoroughly tested.
fn allocate_section_mappings(elf_file: &ElfFile) -> Result<SectionMappings, &'static str> {
    let (executable_len, read_only_len, read_write_len): (usize, usize, usize) = {
        let mut executable_len = 0;
        let mut read_only_len = 0;
        let mut read_write_len = 0;

        for (section_index, section) in elf_file.section_iter().enumerate() {
            let section_flags = section.flags();

            // Skip non-allocated sections; they don't need to be loaded into memory.
            if section_flags & SHF_ALLOC == 0 {
                continue;
            }

            let name = section.get_name(elf_file);

            // Zero-sized sections may be aliased references to the next section in the ELF
            // file, but only if they have the same offset. Ignore the empty .text section
            // at the start.
            let section = if section.size() == 0 && name != Ok(".text") {
                let next_sec = elf_file
                    .get_section_header((section_index + 1) as u16)
                    .map_err(|_| "couldn't get next section for a zero-sized section")?;
                if next_sec.offset() == section.offset() {
                    next_sec
                } else {
                    section
                }
            } else {
                section
            };

            let size = section.size() as usize;
            let align = section.align() as usize;
            let offset = section.offset() as usize;
            let addend = size.next_multiple_of(align);

            let is_write = section_flags & SHF_WRITE == SHF_WRITE;
            let is_exec = section_flags & SHF_EXECINSTR == SHF_EXECINSTR;
            let is_tls = section_flags & SHF_TLS == SHF_TLS;

            // .text
            if is_exec {
                executable_len = executable_len.max(offset + addend);
            }
            // .tdata (.tbss sections are ignored)
            else if is_tls {
                if section.get_type() == Ok(SectionHeaderType::ProgBits) {
                    read_only_len += addend;
                }
            }
            // .bss and .data
            else if is_write {
                read_write_len += addend;
            }
            // .rodata, .eh_frame, and .gcc_except_table
            else {
                read_only_len += addend;
            }
        }

        (executable_len, read_only_len, read_write_len)
    };

    // HACK: Mappings should be optional, this is just a workaround for the
    //       possibility of an empty mapping.
    let executable_len = executable_len.max(1);
    let read_only_len = read_only_len.max(1);
    let read_write_len = read_write_len.max(1);

    Ok(SectionMappings {
        executable: MemoryMap::alloc_zeroed(executable_len, MapFlags::READ_WRITE_EXEC)?,
        read_only: MemoryMap::alloc_zeroed(read_only_len, MapFlags::READ_WRITE)?,
        read_write: MemoryMap::alloc_zeroed(read_write_len, MapFlags::READ_WRITE)?,
    })
}

struct SectionMappings {
    executable: MemoryMap,
    read_only: MemoryMap,
    read_write: MemoryMap,
}



pub fn crate_names_in_symbol(symbol_name: &str) -> Vec<&str> {
    let mut ranges = crate_name_ranges_in_symbol(symbol_name);
    ranges.dedup();

    ranges
        .into_iter()
        .filter_map(|range| symbol_name.get(range))
        .collect()
}

fn crate_name_ranges_in_symbol(symbol_name: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut start_bound = Some(0);
    while let Some(start) = start_bound {
        // The crate name will be right before the first occurrence of "::".
        let end = symbol_name
            .get(start..)
            .and_then(|s| s.find("::"))
            .map(|end_index| start + end_index);

        // If the substring (start..end) contains " as ", skip it and let the next
        // iteration of the loop handle it to avoid counting it twice.
        if let Some(end) = end {
            let substring = symbol_name.get(start..end);
            if substring.is_some_and(|s| !s.contains(" as ")) {
                // Find the beginning of the crate name, searching backwards from `end`. If
                // there was no non-name character, then the crate name started at the beginning
                // of `substring`.
                let start = substring
                    .and_then(|s| s.rfind(|ch: char| !(ch.is_alphanumeric() || ch == '_')))
                    // Move forward to the actual start of the crate name.
                    .map(|start_index| start + start_index + 1)
                    .unwrap_or(start);

                ranges.push(start..end);
            }
        }

        // Advance to the next substring.
        start_bound = symbol_name
            .get(start..)
            .and_then(|s| s.find(" as "))
            .map(|start_index| start + start_index + " as ".len());
    }

    ranges
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_names_from_symbol_names() {
        macro_rules! check {
            ($sym:literal == [$($name:literal),*]) => {
                assert_eq!(crate_names_in_symbol($sym), vec![$($name),*] as Vec<&str>);
            };
        }

        check!("foo::Bar" == ["foo"]);
        check!("foo::bar::Thing" == ["foo"]);
        check!("<foo::Foo as bar::Bar>::run" == ["foo", "bar"]);
        check!("<usize as bar::Foo>::do_something" == ["bar"]);
        check!("std::ops::Range::<u32>::from" == ["std"]);
        check!("<alloc::boxed::Box<T>>::into_inner" == ["alloc"]);
        check!("u64" == []);
    }

    #[test]
    fn searching() {
        let loader = Loader::new("tests/output");
        assert_eq!(
            loader.find_object_files("add_"),
            Ok(vec!["add_one.o".to_string()]),
        );
    }
}
