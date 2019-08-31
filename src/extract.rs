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

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum Context {
    Title,
    Term,
    Id,
    Text,
}

#[derive(Debug)]
struct ContextBuilder {
    context_stack: StackPosition<Vec<(Context, String)>>,
    collection_stack: StackPosition<Context>,
    in_progress: Option<String>,
}

impl ContextBuilder {
    fn new() -> Self {
        Self {
            context_stack: StackPosition::new(),
            collection_stack: StackPosition::new(),
            in_progress: Some(String::new()),
        }
    }

    pub fn collect_for_siblings<'a>(&mut self, ctx: Context, id: Option<&'a str>) {
        self.enter();

        let parent = self.collection_stack.parent_mut().expect("Should never be without parental context");
        //if parent.is_some() {
        //    panic!("Oh gosh, collecting parental context but we're already collecting parental context.");
        //}
        *parent = ctx;
        //println!("context for parent: {:#?}", parent);

        // add context to the parent element
        if let Some(id) = id {
            let parent = self.context_stack.parent_mut().expect("Should never be without a parent in the context stack");
            parent.push((Context::Id, id.to_string()));
            //println!("id for parent: {:#?}", parent);
        }
    }

    pub fn collect_for_children<'a>(&mut self, ctx: Context, id: Option<&'a str>) {
        self.enter();

        let current = self.collection_stack.current_mut().expect("Should never be without parental context");
        *current = ctx;

        if let Some(id) = id {
            let current = self.context_stack.current_mut().expect("Should never be without a current in the context stack");
            current.push((Context::Id, id.to_string()));
        }
    }

    pub fn id_for_children<'a>(&mut self, id: &'a str) {
        self.enter();
        // add context to the parent element
        if let Some(current) = self.context_stack.current_mut() {
            current.push((Context::Id, id.to_string()));
            //println!("id for children: {:#?}", current);
        }
    }

    pub fn enter(&mut self) {
        self.reset_in_progress();
        self.context_stack.enter(vec![]);
        self.collection_stack.enter(Context::Text);
        self.dive();
    }

    pub fn dive(&mut self) {
        self.context_stack.dive();
        self.collection_stack.dive();
    }

    pub fn record(&mut self, data: &str) {
        // println!("Recorded text '{:#?}' for context {:#?}", data, self.collection_stack.current());
        self.in_progress.as_mut().unwrap().push_str(data);
    }

    pub fn reset_in_progress(&mut self) -> Option<()> {
        match self.collection_stack.current_mut()? {
            Context::Title => {
                let inprogress = self.in_progress.take().unwrap();
                self.in_progress = Some(String::new());

                self.context_stack.parent_mut().unwrap().push((Context::Title, inprogress));
            },
            Context::Term => {
                let inprogress = self.in_progress.take().unwrap();
                self.in_progress = Some(String::new());

                self.context_stack.parent_mut().unwrap().push((Context::Term, inprogress));
            },
            Context::Text => {
                let inprogress = self.in_progress.take().unwrap();
                self.in_progress = Some(String::new());

                self.context_stack.current_mut().unwrap().push((Context::Text, inprogress));
            },
            Context::Id => {
                unreachable!("wat?");
            },
        }
        Some(())
    }

    pub fn surface(&mut self) -> Option<Vec<(Context, String)>> {
        self.reset_in_progress()?;
        let ctx = self.context_stack.surface()?;

        // Only return data if the current context has an ID, otherwise
        // re-home it to the parent context
        if ctx.iter().any(|(ctx, _)| ctx == &Context::Id) {
            Some(ctx)
        } else {
            self.context_stack.current_mut().unwrap().extend(ctx);
            None
        }
    }
}

#[derive(Debug)]
struct StackPosition<T: std::cmp::Eq + std::fmt::Debug + std::hash::Hash + std::clone::Clone> {
    stack: Vec<T>,
    depth: HashMap<usize, u8>,
}

impl <T: std::fmt::Debug + std::cmp::Eq + std::hash::Hash + std::clone::Clone>StackPosition<T> {
    fn new() -> StackPosition<T> {
        StackPosition {
            stack: vec![],
            depth: HashMap::new(),
        }
    }

    fn enter(&mut self, value: T) {
        self.stack.push(value.clone());
        let len = self.stack.len();
        self.depth.entry(len).or_insert(0);
    }

    fn dive(&mut self) {
        let pos = self.stack.len();
        if let Some(top) = self.stack.last() {
            let depth = self.depth.get_mut(&pos).unwrap();
            //println!("Depth: {:#?}", depth);
            *depth += 1
        }
    }

    fn current(&self) -> Option<&T> {
        self.stack.last()
    }

    fn current_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    fn parent_mut(&mut self) -> Option<&mut T> {
        let len = self.stack.len();
        if len == 0 {
            None
        } else {
            self.stack.get_mut(len - 1)
        }
    }

    fn surface(&mut self) -> Option<T> {
        let pos = self.stack.len();
        if let Some(top) = self.stack.last() {
            let depth = self.depth.get_mut(&pos).unwrap();
            *depth -= 1;
            //println!("Surfacing from top: {:#?} with depth: {:#?}", top, depth);

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

#[derive(Debug)]
pub struct IndexBuilder {
    index: Index,

    context_builder: ContextBuilder,

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

            context_builder: ContextBuilder::new(),

            file_map: file_map,
        }
    }

    fn load(&mut self, document: &Path) {
        let file = File::open(document).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);

        let mut seen: u64 = 0;
        for event in parser {
            seen += 1;
            if false  && seen > 100 {
                println!("{:#?}", self);
                panic!();
            }
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
        // println!("Starting {:?}#", name);
        let id: Option<&str> = attributes
            .iter()
            .filter(|attr|
                    attr.name.local_name == "id"
                    && attr.name.namespace
                    .as_ref()
                    .map(|s| s.as_ref()) == Some("http://www.w3.org/XML/1998/namespace")
            ).map(|attr| attr.value.as_ref())
            .next();

        match (name.local_name.as_ref(), id) {
            ("title", id) => self.context_builder.collect_for_siblings(Context::Title, id),
            ("term", id) => self.context_builder.collect_for_siblings(Context::Term, id),
            (_, id) => self.context_builder.collect_for_children(Context::Text, id),
            // (_, None) => self.context_builder.collect_for_children(Context::Text, None),
        }

        if name.local_name == "include"
            && name.namespace == Some("http://www.w3.org/2001/XInclude".to_string())
        {
            panic!("We don't support xinclude for {:#?}!", attributes);
        }
    }

    fn handle_end_element(&mut self, _name: OwnedName) {
        //println!("Ending {:?}#", _name);
        if let Some(ctx) = self.context_builder.surface() {
            println!("Context: {:#?}", ctx);
        }

        /*

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
        */
    }
    fn handle_characters(&mut self, text: String) {
        self.context_builder.record(&text);
    }

}
