extern crate glob;
extern crate xml;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::env;
use std::process::exit;
mod extract;
mod fileindex;

// For exit codes in use:
// https://www.freebsd.org/cgi/man.cgi?query=sysexits<Paste>

fn usage() {
    // argv0 is always expected.
    let name = env::args().nth(0).unwrap();
    println!(r#"
Usage: {} [options] <input.xml> <generated dir> <output file>
  <input.xml>     The input XML file
  <generated dir> The directory containing generated HTML documentation
  <output file>   The file to write the index

  --json    outputs as json
"#, name);
}

fn main() {
    // Arguments parsing

    // This is a naïve approach to parsing arguments. This only works as
    // long as `--name value` is not required.
    let (flags, mut non_flags): (Vec<String>, Vec<String>)
        = env::args().partition(|arg| arg.chars().next().unwrap() == '-');

    // Removes argv0.
    non_flags.remove(0);

    // First checks whether we print usage.
    if flags.iter().any(|arg| arg == "--help") {
        usage();
        exit(0);
    }

    // Then sanity checks we have all required arguments.
    if non_flags.len() != 3 {
        eprintln!("Error: Missing {} mandatory arguments.", 3 - non_flags.len());
        usage();
        exit(64);
    }

    // Consume parameters
    let input_xml = non_flags.remove(0);
    let input_html = non_flags.remove(0);
    let output = non_flags.remove(0);

    // Then checks for flags
    let json = flags.iter().any(|arg| arg == "--json");

    let mut file = File::create(output).expect("Failed to open the output file");

    let file_index = fileindex::IndexMap::interpret_from(Path::new(&input_html));
    println!("Loaded {} ID-to-file mappings", file_index.len());
    let index = extract::IndexBuilder::build_from(
        Path::new(&input_xml),
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
