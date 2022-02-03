use std::collections::HashMap;
use std::fs;
use roxmltree;

pub enum DataType {
    Int,
    Float,
    Text,
    Bool,
    Link,
    Timestamp,
}

impl DataType {
    pub fn is_numeric(&self) -> bool {
        match *self {
            Self::Int | Self::Float => true,
            _ => false
        }
    }
}

pub struct Field {
    name: String,
    label: String,
    datatype: DataType,
    i18n: bool,
    array_pos: usize,
    is_virtual: bool, // vim at least thinks 'virtual' is reserved
}

pub struct Link {
    field: String,
    reltype: String,
    key: String,
    map: String,
    class: String,
}

pub struct Class {
    class: String,
    label: String,
    fields: HashMap<String, Field>,
    links: HashMap<String, Link>,
}

pub struct Parser {
    classes: HashMap<String, Class>,
}

impl Parser {

    pub fn new() -> Self {
        Parser {
            classes: HashMap::new(),
        }
    }

    pub fn parse_file(file_name: &str) -> Parser {
        let xml = fs::read_to_string(file_name).unwrap();
        Parser::parse_string(&xml)
    }

    pub fn parse_string(xml: &str) -> Parser {

        let doc = roxmltree::Document::parse(xml).unwrap(); // TODO errors

        let mut parser = Parser::new();

        for root_node in doc.root().children() {
            if root_node.tag_name().name() == "IDL" {
                for class_node in root_node.children() {
                    if  class_node.node_type() == roxmltree::NodeType::Element
                        && class_node.tag_name().name() == "class" {
                        parser.add_class(&class_node);
                    }
                }
            }
        }

        parser
    }

    fn add_class(&mut self, node: &roxmltree::Node) {
        println!("Adding class {}", node.attribute("id").unwrap());
    }
}


