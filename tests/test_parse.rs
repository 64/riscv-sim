use aca::{inst::Label, program::Program};
use std::collections::HashMap;

#[test]
fn parse_all() {
    for entry in std::fs::read_dir("asm").unwrap() {
        let entry = entry.unwrap();
        let contents = std::fs::read_to_string(entry.path()).unwrap();
        let prog_name = entry.file_name().to_str().unwrap().to_owned();

        println!("parsing {prog_name}...");
        contents
            .parse::<Program>()
            .expect(&format!("failed to parse program {}", prog_name));
    }
}

#[test]
fn check_labels() {
    let contents = std::fs::read_to_string("asm/label.asm").unwrap();
    let prog = contents
        .parse::<Program>()
        .expect("failed to parse asm/label.asm");

    let mut test = HashMap::new();
    test.insert(Label("foo".to_owned()), 1);
    test.insert(Label(".bar".to_owned()), 3);
    test.insert(Label("baz5".to_owned()), 4);
    assert_eq!(prog.labels, test);
}
