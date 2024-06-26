use std::fmt;
use std::io::{Cursor, Read};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use xmltree::{Element, XMLNode};
use xmltree::EmitterConfig;
use serde::de::{self, Visitor};
use sha2::{Sha256, Digest};
use pathetic;
use url::Url;

use crate::error::{Errors};

// echo -n "text" | sha256sum
const TEXT_NODE_HASH: &str = "982d9e3eb996f559e633f4d194def3761d909f5a3b647d1a851fead67c32c9d1";

#[derive(Clone, Debug)]
pub struct Xml {
    pub element: Option<Element>,
    pub text: Option<String>,
}

pub fn xml_to_hash(xml: &Xml) -> String {
    if xml.is_text() {
        return TEXT_NODE_HASH.to_string();
    }

    let mut hasher = Sha256::new();

    let mut hasher_items = Vec::new();
    hasher_items.push("TAG:".to_owned() + &xml.get_element_tag_name());

    for (attribute, value) in xml.element.clone().unwrap().attributes {
        hasher_items.push("ATTRIBUTE:".to_owned() + &attribute.clone());

        if attribute == "href" {
            let mut parts = url_to_hash_parts(&value);
            for part in parts {
                hasher_items.push("HREF:".to_owned() + &part);
            }
        }
    }

    hasher_items.sort();

    hasher.update(hasher_items.join(""));

    format!("{:x}", hasher.finalize())
}

pub fn combine_xml(parent: &Xml, child: &Xml) -> Xml {
    let mut combined = parent.element.clone().expect("Parent XML is not an Element");

    if let Some(child_element) = &child.element {
        combined.children.push(xmltree::XMLNode::Element(child_element.clone()));
    }

    if let Some(child_text) = &child.text {
        combined.children.push(xmltree::XMLNode::Text(child_text.clone()));
    }

    Xml {
        element: Some(combined),
        text: None,
    }
}

impl Serialize for Xml {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct XmlVisitor;

impl<'de> Visitor<'de> for XmlVisitor {
    type Value = Xml;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("valid XML in a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Ok(element) = Element::parse(value.as_bytes()) {
            Ok(Xml {
                element: Some(element),
                text: None,
            })
        } else {
            Ok(Xml {
                element: None,
                text: Some(value.to_owned()),
            })
        }
    }
}

impl<'de> Deserialize<'de> for Xml {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(XmlVisitor)
    }
}

impl fmt::Display for Xml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_string())
    }
}

impl Xml {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Xml, Errors> {
        match Element::parse(reader) {
            Ok(element) => {
                let xml = Xml {
                    element: Some(element),
                    text: None,
                };
                
                Ok(xml)
            }
            _ => {
                Err(Errors::XmlParseError)
            }
        }
    }

    pub fn without_children(&self) -> Xml {
        if self.element.is_some() {

            let mut copy = self.clone();

            copy.element.as_mut().unwrap().children.clear();

            copy

        } else {
            self.clone()
        }
    }

    pub fn from_void() -> Xml {
        Xml {
            element: None,
            text: None,
        }
    }
}

impl Xml {
    pub fn get_element_tag_name(&self) -> String {
        if let Some(element) = &self.element {
            return element.name.clone();
        }

        "".to_string()
    }

    pub fn get_attributes(&self) -> Vec<String> {
        if let Some(element) = &self.element {
            return element.attributes.keys().cloned().collect();
        }

        Vec::new()
    }

    pub fn get_attribute_value(&self, name: &str) -> Option<String> {
        if self.element.is_none() {
            log::warn!("Attempting to get attribute: {} on Xml, but xml is not an element", name);
            return None;
        }

        self.element.clone().unwrap().attributes.get(name).cloned()
    }

    pub fn get_children(&self) -> Vec<Xml> {
        if let Some(element) = &self.element {
            return element.children.iter().filter_map(|child| {
                match child {
                    XMLNode::Element(child_element) => {
                        let xml = Xml {
                            element: Some(child_element.clone()),
                            text: None,
                        };

                        Some(xml)
                    },
                    XMLNode::Text(child_text) => {
                        let xml = Xml {
                            element: None,
                            text: Some(child_text.to_string()),
                        };

                        Some(xml)
                    },
                    _ => None,
                }

            }).collect();
        }

        Vec::new()
    }

    pub fn has_children(&self) -> bool {
        !&self.get_children().is_empty()
    }

    pub fn is_text(&self) -> bool {
        self.text.is_some()
    }

    pub fn is_element(&self) -> bool {
        self.element.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.element.is_none()
    }

    pub fn is_script_element(&self) -> bool {
        if let Some(element) = &self.element {
            return element.name == "script";
        }

        false
    }

    pub fn to_string(&self) -> String {
        if let Some(element) = &self.element {
            format!("{}", &element_to_string(&element))
        } else if let Some(text) = &self.text {
            format!("{}", &text)
        } else {
            format!("{}", "")
        }
    }

    pub fn to_string_with_child_string(&self, content: String) -> Result<String, String> {
        if let Some(element) = &self.element {
            let result = format!(
                "{}{}{}",
                get_opening_tag(&element),
                content,
                get_closing_tag(&element),
            );

            Ok(result)
        } else {
            Err("XML is not an Element; it can't take any children".to_string())
        }
    }

    pub fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.to_string());

        format!("{:x}", hasher.finalize())
    }

    pub fn is_equal(&self, xml: Xml) -> bool {
        if let Some(element_a) = &self.element {
            if let Some(element_b) = xml.element {
                if element_a.name != element_b.name {
                    return false;
                }

                let attributes_a = &element_a.attributes;
                let attributes_b = &element_b.attributes;

                if attributes_a.keys().len() != attributes_b.keys().len() {
                    return false;
                }

                for (attribute, value_a) in attributes_a {
                    let value_b = attributes_b.get(attribute);

                    if let Some(value_b) = value_b {
                        if value_a != value_b {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

            } else {
                return false;
            }

        } else if let Some(text_a) = &self.text {
            if let Some(text_b) = xml.text {
                return text_a == &text_b;
            } else {
                return false;
            }
        }

        true
    }
}

fn element_to_string(element: &Element) -> String {
     let mut config = EmitterConfig::new();
     config.write_document_declaration = false;

     let mut cursor = Cursor::new(Vec::new());

     element.write_with_config(&mut cursor, config).unwrap();

     let serialized_xml = String::from_utf8(cursor.into_inner()).unwrap();

     // TODO
     let serialized_xml = serialized_xml.replace(" xmlns=\"http://www.w3.org/1999/xhtml\"", "");

     serialized_xml
}

pub fn get_opening_tag(element: &Element) -> String {
    let mut tag = format!("<{}", element.name);

    for (attr, value) in &element.attributes {
        tag.push_str(&format!(" {}=\"{}\"", attr, value.replace("\"", "&quot;")));
    }
    tag.push('>');

    tag
}

pub fn get_closing_tag(element: &Element) -> String {
    format!("</{}>", element.name)
}

fn url_to_hash_parts(url: &str) -> Vec<String> {
    // This is how I'm checking if a URL is absolute...
    // pathetic is a library for dealing with relative URLs
    if Url::parse(url).is_ok() {
        return Vec::new();
    }

    let parsed_url = pathetic::Uri::new(url).expect("Failed to parse the URL");

    let mut output = Vec::new();

    let path = parsed_url.path_segments().collect::<Vec<_>>().join("/");
    output.push(path);

    if let Some(query) = parsed_url.query() {
        for param in query.split('&') {
            if let Some((name, _)) = param.split_once('=') {
                output.push(name.to_string());
            } else {
                output.push(param.to_string());
            }
        }
    }

    output
}
