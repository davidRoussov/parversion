use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use quick_js::{Context, JsValue};

use crate::prelude::*;
use crate::id::{ID};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Runtime {
    AWK,
    NodeJS,
    Python,
    QuickJS,
}

trait Transform {
    fn get_id(&self) -> ID;
    fn get_runtime(&self) -> Runtime;
    fn get_code(&self) -> String;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonSchemaTransform {
    id: ID,
    runtime: Runtime,
    code: String,
    source: String,
    target: String,
}

impl Transform for JsonSchemaTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeFieldsTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataNodeFieldsTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeHashTransform {
    id: ID,
    runtime: Runtime,
    regex: String,
    code: String,
}

impl Transform for DataNodeHashTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeRecursiveTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataNodeRecursiveTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataToJsonFieldTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataToJsonFieldTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Transformation {
    DataNodeFieldsTransform(DataNodeFieldsTransform),
    DataNodeRecursiveTransform(DataNodeRecursiveTransform),
    DataNodeHashTransform(DataNodeHashTransform),
    DataToJsonFieldTransform(DataToJsonFieldTransform),
    JsonSchemaTransform(JsonSchemaTransform),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct XMLElementTransformation {
    pub id: ID,
    pub description: String,
    pub runtime: Runtime,
    pub infix: String,
}

impl XMLElementTransformation {
    fn prefix(&self, element: String, attributes: HashMap<String, String>) -> String {
        let element_code = format!("let element = '{}';", element);

        let attributes_code = {
            let attributes_list: Vec<String> = attributes
                .into_iter()
                .map(|(key, value)| format!("'{}': '{}'", key, value))
                .collect();
            format!("let attributes = {{ {} }};", attributes_list.join(", "))
        };

        format!("{}\n{}", element_code, attributes_code)
    }

    fn suffix(&self) -> String {
        match self.runtime {
            Runtime::QuickJS => {
                format!("JSON.stringify({{ element, attributes }})")
            },
            _ => panic!("unexpected runtime: {:?}", self.runtime),
        }
    }
    
    pub fn transform(
        &self,
        element: String,
        attributes: HashMap<String, String>
    ) -> (
        Option<String>,
        HashMap<String, String>
    ) {
        log::trace!("In transform");

        let prefix = self.prefix(element, attributes);
        let suffix = self.suffix();

        let code = format!("{}\n{}\n{}", prefix, self.infix, suffix);

        match self.runtime {
            Runtime::QuickJS => {
                let quick_context = Context::new().unwrap();

                let result =  quick_context.eval_as::<String>(&code).unwrap();

                let parsed: Value = serde_json::from_str(&result).unwrap();

                let transformed_element = parsed.get("element").and_then(|e|
                    e.as_str().map(String::from));

                let transformed_attributes = parsed.get("attributes")
                    .and_then(|attr| attr.as_object())
                    .map(|attr_obj| {
                        attr_obj.iter().map(|(k, v)| {
                            (k.clone(), v.as_str().unwrap_or("").to_string())
                        }).collect::<HashMap<String, String>>()
                    }).unwrap_or_default();

                (transformed_element, transformed_attributes)
            },
            _ => panic!("Unexpected runtime: {:?}", self.runtime),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DocumentTransformation {
    XMLElementTransformation(XMLElementTransformation),
}

//lazy_static! {
//    pub static ref VALUE_TRANSFORMATIONS: Vec<Transformation> = vec![
//        Transformation {
//            runtime: Runtime::AWK,
//            description: String::from("Converts American weights in pounds (lbs)"),
//            regex: r"\b\d+(\.\d+)?\s*(lbs?|pounds?)\b",
//            code: String::from(r#"{ printf "%.2f lbs = %.2f kg\n", $1, $1 * 0.45359237 }"#),
//        },
//        Transformation {
//            runtime: Runtime::AWK,
//            description: String::from("Identity Transformation"),
//            regex: r"(?s).*",
//            code: String::from(r#"{ print $0 }"#),
//        },
//    ];
//}
