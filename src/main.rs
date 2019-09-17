extern crate glob;
extern crate xml;
use std::io::Write;
use std::fs::File;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use std::process::exit;
mod extract;
mod fileindex;


/// Index a Docbook file for use with elasticlunr, for Nixpkgs
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// Output the index as JSON. Incompatible with --anchors
    #[structopt(long)]
    json: bool,

    /// Index available anchors. Incompatible with --json
    #[structopt(long)]
    anchors: bool,

    /// The input XML file
    input_xml: PathBuf,

    /// The directory containing generated HTML documentation
    generated_dir: PathBuf,

    /// The file to write the index
    output_file: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    // Last sanity checks
    if opt.json && opt.anchors {
        // But why?? For backwards compatibility purpose.
        // The json document is *only* the elasticlunr index.
        // Meanwhile, the js document was setting a global variable.
        // It's trivial to add another global variable it without causing issues.
        eprintln!("Error: --json and --anchors are not supported together.");
        exit(64);
    }

    let mut file = File::create(opt.output_file).expect("Failed to open the output file");

    let file_index = fileindex::IndexMap::interpret_from(Path::new(&opt.generated_dir));
    println!("Loaded {} ID-to-file mappings", file_index.len());
    let index = extract::IndexBuilder::build_from(
        Path::new(&opt.input_xml),
        file_index.clone()
    );

    if opt.json {
        file.write_all(index.to_json_pretty().as_bytes()).expect("failed to write the index");
    } else {
        write!(file, "window.searchIndexData = {};\n", index.to_json())
            .expect("failed to write the text index");
        if opt.anchors {
            let json = serde_json::to_string(&file_index.clone()).unwrap();
            write!(file, "window.anchorsIndex = {};\n", json)
                .expect("failed to write the anchors index");
        }
    }
}
