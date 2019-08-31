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

struct TitleBuilder {
    title: Option<String>,
    position: StackPosition<()>
}

impl TitleBuilder {
    fn new() -> TitleBuilder {
        TitleBuilder {
            title: None,
            position: StackPosition::new(),
        }
    }

    fn in_title(&self) -> bool {
        self.title.is_some() &&
        self.position.current().is_some()
    }

    fn enter(&mut self) {
        if self.in_title() {
            panic!("Entering a new title when we are already in a title");
        }

        self.position.enter(());
        self.title = Some(String::new());
        self.position.dive();
    }

    fn dive(&mut self) {
        self.position.dive();
    }

    fn surface(&mut self) -> Option<String> {
        if self.in_title() && self.position.surface().is_some() {
            let title = self.title.take();
            self.title = Some(String::new());
            title
        } else {
            None
        }
    }

    fn record(&mut self, fragment: &str) {
        if let Some(ref mut title) = &mut self.title {
            title.extend(fragment.chars()); // !!! what is the way to do this?
        } else {
            panic!("Tried to record title bytes without being in a title");
        }
    }


}

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

    title_builder: TitleBuilder,
    titles: StackPosition<String>,

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

            title_builder: TitleBuilder::new(),
            titles: StackPosition::new(),

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
        match (self.title_builder.in_title(), name.local_name.as_ref()) {
            (false, "title") => self.title_builder.enter(),
            (true, "title") => unreachable!("Entered a title while already in a title!"),
            (true, _) => self.title_builder.dive(),
            (false, _) => (),
        }

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
        if let Some(title) = self.title_builder.surface() {
            self.titles.enter(title);
            self.titles.dive();
            // Dive an extra time because titles are for the content
            // after the </title>. Note this is needed because later
            // we unconditionally titles.surface ()
            self.titles.dive();
        }

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
        if self.title_builder.in_title() {
            self.title_builder.record(&text);
        } else if let Some(id) = self.ids.current() {
            let stored_txt = self.id_text.entry(id.clone())
                .or_insert(String::from(""));

            stored_txt.push_str(" ");
            stored_txt.push_str(&text);
        } else {
            println!("WARNING: Losing orphaned text: {:?}", text);
        }
    }

}
