use glob::glob;
use std::fs::{File};
use std::path::{Path, PathBuf};
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use xml::attribute::OwnedAttribute;

pub type Map = HashMap<String, PathBuf>;

pub struct IndexMap {
}

fn get_attr<'a>(name: &str, attributes: &'a Vec<OwnedAttribute>) -> Option<&'a str> {
    for attr in attributes {
        if attr.name.local_name == name {
            return Some(&attr.value);
        }
    }

    None
}

impl IndexMap {
    pub fn interpret_from(root: &Path) -> Map {
        let mut map: Map = HashMap::new();
		let patt = root.join("**/*.html");
        for entry in glob(patt.to_str().expect("while parsing a path")).unwrap() {
            println!("{:?}", entry);
            let entry = entry.unwrap();
            let parsed = IndexMap::parse(&entry);
            for id in parsed {
                map.insert(id, entry.clone());
            }
        }
        map
    }

    pub fn parse(document: &Path) -> Vec<String> {
        let file = File::open(document);

        let file = BufReader::new(file.unwrap());

        let mut id_list: Vec<String> = vec![];

        let parser = EventReader::new(file);
        for event in parser {
            match event {
                Ok(XmlEvent::StartElement { attributes, .. }) => {
                    if let Some(id) = get_attr("id", &attributes) {
                        id_list.push(id.to_string());
                    }
                },
                Ok(XmlEvent::EndElement { .. }) => { },
                Ok(XmlEvent::Characters (_)) => { },
                Ok(XmlEvent::Whitespace(_)) => { },
                Ok(XmlEvent::StartDocument { .. }) => { },
                Ok(XmlEvent::EndDocument { .. }) => { },
                Ok(XmlEvent::ProcessingInstruction { .. }) => { }
                Ok(XmlEvent::Comment(_)) => { }
                Ok(XmlEvent::CData(_)) => { }
                Err(e) => {
                    panic!("Error: {}", e);
                }
            }
        }

        id_list
    }
}
