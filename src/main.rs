extern crate glob;
extern crate xml;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::env;
mod extract;
mod fileindex;

fn main() {
    let input_xml = env::args()
        .nth(1)
        .expect("Arg 1 must be the input XML file");

    let input_html = env::args()
        .nth(2)
        .expect("Arg 2 must be the directory containing generated HTML documentation");

    let output = env::args()
        .nth(3)
        .expect("Arg 3 must be the file to write the index");

    let mut file = File::create(output).expect("Failed to open the output file");

    let file_index = fileindex::IndexMap::interpret_from(Path::new(&input_html));
    println!("Loaded {} ID-to-file mappings", file_index.len());
    let index = extract::IndexBuilder::build_from(
        Path::new(&input_xml),
        file_index
    );

    file.write_all(index.to_json().as_bytes()).expect("failed to write the index");
}