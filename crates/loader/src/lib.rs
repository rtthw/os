//! # Loader
//!
//! The runtime [linker]/[loader].
//!
//! [linker]: https://en.wikipedia.org/wiki/Linker_(computing)
//! [loader]: https://en.wikipedia.org/wiki/Loader_(computing)

#![no_std]

// #[macro_use]
extern crate alloc;

use {
    abi::elf::{ElfFile, ObjectFileType},
    alloc::{
        collections::BTreeSet,
        sync::{Arc, Weak},
    },
    hashbrown::HashMap,
    spin::Mutex,
};



/// A set of loaded [objects](LoadedObject) and [sections](LoadedSection).
pub struct Loader {
    objects: Mutex<HashMap<Arc<str>, Arc<LoadedObject>>>,
    sections: Mutex<HashMap<Arc<str>, Weak<LoadedSection>>>,
}

/// An object that has been loaded into memory.
pub struct LoadedObject {
    /// The demangled name of this object.
    pub name: Arc<str>,
    /// The sections that have been loaded into memory for this object.
    pub sections: HashMap<usize, Arc<LoadedSection>>,
    /// A set of section indices representing the global sections of this
    /// object. They can be used as keys for [Self::sections].
    pub global_sections: BTreeSet<usize>,
}

/// An object section that has been loaded into memory.
pub struct LoadedSection {
    /// The demangled name of this section.
    pub name: Arc<str>,
    /// The object that contains this section.
    pub owner: Weak<LoadedObject>,
    /// Whether this section is global (public).
    pub global: bool,
}



impl Loader {
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            sections: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_object(&self, name: &str) -> Option<Weak<LoadedObject>> {
        self.objects.lock().get(name).map(Arc::downgrade)
    }

    pub fn get_section(&self, name: &str) -> Option<Weak<LoadedSection>> {
        self.sections.lock().get(name).cloned()
    }

    pub fn load_object(
        &self,
        object_bytes: &[u8],
    ) -> Result<Arc<Mutex<LoadedObject>>, &'static str> {
        let (object, elf_file) = self.load_object_sections(object_bytes)?;
        self.add_sections(object.lock().sections.values());
        self.relocate_object_sections(&elf_file, &object)?;

        Ok(object)
    }

    fn load_object_sections<'obj>(
        &self,
        object_bytes: &'obj [u8],
    ) -> Result<(Arc<Mutex<LoadedObject>>, ElfFile<'obj>), &'static str> {
        let elf_file = ElfFile::new(object_bytes)?;
        if elf_file.header.get_type() != ObjectFileType::Relocatable {
            return Err("not a relocatable ELF file");
        }

        Err("TODO: Linker::load_object_sections")
    }

    fn relocate_object_sections(
        &self,
        _elf_file: &ElfFile,
        _object: &Arc<Mutex<LoadedObject>>,
    ) -> Result<(), &'static str> {
        Err("TODO: Linker::relocate_object_sections")
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
