use loader::*;

#[test]
fn add_one() {
    let bytes = include_bytes!("output/add_one.o").to_vec();
    let loader = Loader::new();
    let _object = loader.load_object("add_one", &bytes).unwrap();
    let section = loader
        .get_section_ending_with("add_one")
        .unwrap()
        .upgrade()
        .unwrap();

    let out = unsafe { section.as_function::<fn(i32) -> Option<i32>>() }.unwrap()(3);
    assert_eq!(out, Some(4));
}
