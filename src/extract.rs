use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use elasticlunr::Index;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use xml::name::OwnedName;
use xml::namespace::Namespace;
use xml::attribute::OwnedAttribute;
use crate::fileindex;

struct StackPosition<T: std::cmp::Eq + std::hash::Hash + std::clone::Clone> {
    stack: Vec<T>,
    depth: HashMap<T, u8>,
}

impl <T: std::cmp::Eq + std::hash::Hash + std::clone::Clone>StackPosition<T> {
    fn new() -> StackPosition<T> {
        StackPosition {
            stack: vec![],
            depth: HashMap::new(),
        }
    }

    fn enter(&mut self, value: T) {
        self.stack.push(value.clone());
        self.depth.entry(value).or_insert(0);
    }

    fn dive(&mut self) {
        if let Some(top) = self.stack.last() {
            let depth = self.depth.get_mut(top).unwrap();
            *depth += 1
        }
    }

    fn current(&self) -> Option<&T> {
        self.stack.last()
    }

    fn surface(&mut self) -> Option<T> {
        if let Some(top) = self.stack.last() {
            let depth = self.depth.get_mut(top).unwrap();
            *depth -= 1;

            if *depth == 0 {
                Some(self.stack.pop().unwrap())
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct IndexBuilder {
    index: Index,

    ids: StackPosition<String>,
    id_text: HashMap<String, String>,

    titles: StackPosition<String>,

    content_is_title: bool,

    file_map: fileindex::Map,

}

impl IndexBuilder {
    pub fn build_from(document: &Path, file_map: fileindex::Map) -> Index {
        let mut builder = IndexBuilder::new(file_map);
        builder.load(document);
        builder.index
    }

    fn new(file_map: fileindex::Map) -> IndexBuilder {
        IndexBuilder {
            index: Index::new(&["title", "body"]),

            ids: StackPosition::new(),
            id_text: HashMap::new(),

            titles: StackPosition::new(),

            content_is_title: false,

            file_map: file_map,
        }
    }

    fn load(&mut self, document: &Path) {
        let file = File::open(document).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);

        for event in parser {
            match event {
                Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                    self.handle_start_element(name, attributes, namespace);
                }
                Ok(XmlEvent::EndElement { name }) => {
                    self.handle_end_element(name);
                }
                Ok(XmlEvent::Characters(text)) => {
                    self.handle_characters(text);
                }

                // Other event types we don't yet use.
                // We might want to use CData, as it could be used
                // for inline code. Hopefully nobody uses processing
                // instructions.
                Ok(XmlEvent::Whitespace(_)) => { },
                Ok(XmlEvent::StartDocument { .. }) => { },
                Ok(XmlEvent::EndDocument { .. }) => { },
                Ok(XmlEvent::ProcessingInstruction { .. }) => { }
                Ok(XmlEvent::Comment(_)) => { }
                Ok(XmlEvent::CData(_)) => { }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }

    }

    fn handle_start_element(&mut self, name: OwnedName, attributes: Vec<OwnedAttribute>, _namespace: Namespace) {
        self.content_is_title = name.local_name == "title";

        for attr in attributes.iter() {
            if attr.name.local_name == "id"
                && attr.name.namespace == Some(String::from("http://www.w3.org/XML/1998/namespace"))
            {
                self.ids.enter(attr.value.clone());
            }
        }

        self.ids.dive();
        self.titles.dive();

        if name.local_name == "include"
            && name.namespace == Some("http://www.w3.org/2001/XInclude".to_string())
        {
            panic!("We don't support xinclude for {:#?}!", attributes);
        }
    }

    fn handle_end_element(&mut self, _name: OwnedName) {
        if let Some(id) = self.ids.surface() {
            if let Some(text) =  self.id_text.get(&id) {
                let filename = self.file_map.get(&id)
                    .expect(&format!("Somehow, we found an ID ({}) which is not in the output document", id));

                let default = String::from("");
                let title = self.titles.current().unwrap_or(&default);
                println!("title: {:?}", title);

                self.index.add_doc(&format!("{}#{}", filename.display(), id), &[title, text]);
            } else {
                println!("No documentation text found for ID {}", id);
            }
        }

        self.titles.surface();
    }
    fn handle_characters(&mut self, text: String) {
        if self.content_is_title {
            self.titles.enter(text.clone());
            self.titles.dive();
            self.titles.dive(); // ??
        }

        if let Some(id) = self.ids.current() {
            let stored_txt = self.id_text.entry(id.clone())
                .or_insert(String::from(""));

            stored_txt.push_str(" ");
            stored_txt.push_str(&text);
        } else {
            println!("WARNING: Losing orphaned text: {:?}", text);
        }
    }

}
