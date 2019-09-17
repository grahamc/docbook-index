extern crate glob;
extern crate xml;
use std::io::Write;
use std::fs::File;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
mod extract;
mod fileindex;


/// Index a Docbook file for use with elasticlunr, for Nixpkgs
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// Output the index as JSON
    #[structopt(long)]
    json: bool,

    /// The input XML file
    input_xml: PathBuf,

    /// The directory containing generated HTML documentation
    generated_dir: PathBuf,

    /// The file to write the index
    output_file: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let json = opt.json;

    let mut file = File::create(opt.output_file).expect("Failed to open the output file");

    let file_index = fileindex::IndexMap::interpret_from(Path::new(&opt.generated_dir));
    println!("Loaded {} ID-to-file mappings", file_index.len());
    let index = extract::IndexBuilder::build_from(
        Path::new(&opt.input_xml),
        file_index
    );

    if json {
        file.write_all(index.to_json_pretty().as_bytes()).expect("failed to write the index");
    } else {
        file.write_all(r#"
window.searchIndexData = "#.as_bytes()).unwrap();
        file.write_all(index.to_json().as_bytes()).expect("failed to write the index");

        file.write_all(r#"
;
"#.as_bytes()).unwrap();
    }
}
