use loader::*;

struct SearchDir(&'static str);

impl ObjectProvider for SearchDir {
    fn list_objects(&self, prefix: &str) -> Result<Vec<String>, &'static str> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.0).map_err(|_| "failed to read search directory")? {
            let Ok(entry) = entry else {
                continue;
            };
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "found invalid file name in search directory")?;
            if let Some(name) = name.strip_suffix(".o")
                && name.starts_with(prefix)
            {
                names.push(name.to_string());
            }
        }

        Ok(names)
    }

    fn read_object(&self, name: &str) -> Result<Vec<u8>, &'static str> {
        std::fs::read(&format!("{}/{name}.o", self.0)).map_err(|_| "failed to read object")
    }
}

#[test]
fn add_one() {
    let bytes = include_bytes!("output/add_one.o").to_vec();
    let loader = Loader::new(SearchDir("tests/output"));
    let _object = loader.load_object("add_one", &bytes).unwrap();
    let section = loader
        .get_section_ending_with("add_one")
        .unwrap()
        .upgrade()
        .unwrap();

    let func = unsafe { section.as_function::<fn(i32) -> Option<i32>>() }.unwrap();
    assert_eq!(func(3), Some(4));
    assert_eq!(func(-1), Some(0));
}

#[test]
fn dependencies() {
    let bytes = include_bytes!("output/depends_on_add_one.o").to_vec();
    let loader = Loader::new(SearchDir("tests/output"));
    let _object = loader.load_object("depends_on_add_one", &bytes).unwrap();
    let section = loader
        .get_section_ending_with("add_one_to_five")
        .unwrap()
        .upgrade()
        .unwrap();

    let out = unsafe { section.as_function::<fn() -> Option<i32>>() }.unwrap()();
    assert_eq!(out, Some(6));
}
