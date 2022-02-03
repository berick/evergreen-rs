use std::collections::HashMap;
use std::fmt;
use std::fs;
use log::{trace};
use roxmltree;

const OILS_NS_BASE: &str = "http://opensrf.org/spec/IDL/base/v1";
const OILS_NS_OBJ: &str = "http://open-ils.org/spec/opensrf/IDL/objects/v1";
const OILS_NS_OBJ_PREFIX: &str = "oils_obj";
const OILS_NS_PERSIST: &str = "http://open-ils.org/spec/opensrf/IDL/persistence/v1";
const OILS_NS_PERSIST_PREFIX: &str = "oils_persist";
const OILS_NS_REPORTER: &str = "http://open-ils.org/spec/opensrf/IDL/reporter/v1";
const OILS_NS_REPORTER_PREFIX: &str = "reporter";

const AUTO_FIELDS: [&str; 3] = ["isnew", "ischanged", "isdeleted"];

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

impl Into<&'static str> for DataType {
    fn into(self) -> &'static str {
        match self {
            Self::Int 		=> "int",
            Self::Float 	=> "float",
            Self::Text 		=> "text",
            Self::Bool 		=> "bool",
            Self::Timestamp => "timestamp",
            Self::Link 		=> "link",
        }
    }
}

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        match s {
            "int"       => Self::Int,
            "float"     => Self::Float,
            "text"      => Self::Text,
            "bool"      => Self::Bool,
            "timestamp" => Self::Timestamp,
            "link"      => Self::Link,
            _           => Self::Text,
        }
	}
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
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

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field: name={} datatype={} virtual={} label={}",
            self.name, self.datatype, self.is_virtual, self.label)
    }
}

pub enum RelType {
    HasA,
    HasMany,
    MightHave,
    Unset,
}

pub struct Link {
    field: String,
    reltype: RelType,
    key: String,
    map: Option<String>,
    class: String,
}

pub struct Class {
    class: String,
    label: String,
    fields: HashMap<String, Field>,
    links: HashMap<String, Link>,
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Class: class={} fields={} links={} label={} ",
            self.class, self.fields.len(), self.links.len(), self.label)
    }
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

        let name = node.attribute("id").unwrap(); // required

        let label = match node.attribute((OILS_NS_REPORTER, "label")) {
            Some(l) => l.to_string(),
            None => name.to_string(),
        };

        let mut class = Class {
            class: name.to_string(),
            label: label,
            fields: HashMap::new(),
            links: HashMap::new(),
        };

        let mut field_array_pos = 0;

        for child in node.children()
            .filter(|n| n.node_type() == roxmltree::NodeType::Element) {

            if child.tag_name().name() == "fields" {
                for field_node in child.children()
                    .filter(|n| n.node_type() == roxmltree::NodeType::Element)
                    .filter(|n| n.tag_name().name() == "field") {

                    self.add_field(&mut class, field_array_pos, &field_node);
                    field_array_pos += 1;
                }

            } else if child.tag_name().name() == "links" {
                for link_node in child.children()
                    .filter(|n| n.node_type() == roxmltree::NodeType::Element)
                    .filter(|n| n.tag_name().name() == "link") {

                    self.add_link(&mut class, &link_node);
                }
            }
        }

        self.add_auto_fields(&mut class, field_array_pos);

        println!("IDL Adding: {}", class);

        self.classes.insert(class.class.to_string(), class);
    }

    fn add_auto_fields(&self, class: &mut Class, mut pos: usize) {

        for field in AUTO_FIELDS {

            class.fields.insert(field.to_string(), Field {
                name: field.to_string(),
                label: field.to_string(),
                datatype: DataType::Bool,
                i18n: false,
                array_pos: pos,
                is_virtual: true,
            });

            pos += 1;
        }
    }

    fn add_field(&self, class: &mut Class, pos: usize, node: &roxmltree::Node) {

        let label = match node.attribute((OILS_NS_REPORTER, "label")) {
            Some(l) => l.to_string(),
            None => "".to_string(),
        };

        let datatype: DataType =
            match node.attribute((OILS_NS_REPORTER, "datatype")) {
            Some(dt) => dt.into(),
            None => DataType::Text,
        };

        let i18n: bool = match node.attribute((OILS_NS_PERSIST, "i18n")) {
            Some(i) => i == "true",
            None => false,
        };

        let is_virtual: bool = match node.attribute((OILS_NS_PERSIST, "virtual")) {
            Some(i) => i == "true",
            None => false,
        };

        let field = Field {
            name: node.attribute("name").unwrap().to_string(),
            label: label,
            datatype: datatype,
            i18n: i18n,
            array_pos: pos,
            is_virtual: is_virtual,
        };

        class.fields.insert(field.name.to_string(), field);
    }

    fn add_link(&self, class: &mut Class, node: &roxmltree::Node) {

        let link = Link {
            field: node.attribute("field").unwrap().to_string(),
            reltype: RelType::HasA, // TODO
            key: node.attribute("key").unwrap().to_string(),
            map: None, // TODO
            class: node.attribute("class").unwrap().to_string(),
        };

        class.links.insert(link.field.to_string(), link);
    }
}


