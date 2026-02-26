use loader::*;

#[test]
fn add_one() {
    let bytes = include_bytes!("output/add_one.o").to_vec();
    let loader = Loader::new("tests/output");
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
    let loader = Loader::new("tests/output");
    let _object = loader.load_object("depends_on_add_one", &bytes).unwrap();
    let section = loader
        .get_section_ending_with("add_one_to_five")
        .unwrap()
        .upgrade()
        .unwrap();

    let out = unsafe { section.as_function::<fn() -> Option<i32>>() }.unwrap()();
    assert_eq!(out, Some(6));
}
