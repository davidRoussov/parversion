use std::sync::{Arc};
use serde::{Serialize, Deserialize};

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData, bft};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::error::{Errors};
use crate::macros::*;
use crate::node_data::{NodeData};
use crate::node_data_structure::{NodeDataStructure};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traversal {
    pub output_tree: Graph<XmlNode>,
    pub basis_graph: Option<Graph<BasisNode>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValueMetadata {
    pub is_primary_content: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentValue {
    pub meta: ContentValueMetadata,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadataRecursive {
    pub is_root: bool,
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentMetadata {
    pub recursive: Option<ContentMetadataRecursive>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub id: String,
    pub meta: ContentMetadata,
    pub values: Vec<ContentValue>,
    pub children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Output {
    pub data: Content,
}

#[derive(Debug)]
pub enum OutputFormats {
    JSON,
    //XML,
    //CSV
}

const DEFAULT_OUTPUT_FORMAT: OutputFormats = OutputFormats::JSON;

impl Content {
    pub fn remove_empty(&mut self) {
        self.children.iter_mut().for_each(|child| child.remove_empty());
        self.children.retain(|child| !child.is_empty());
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty() && self.children.is_empty()
    }
}














/*********************************************************************
  ---------------------------------------
  Stopgap 
  ---------------------------------------
*********************************************************************/

#[derive(Debug)]
struct XPathStep {
    axis: Option<String>,
    node_test: Option<String>,
    predicates: Vec<String>,
}

fn parse_xpath(expression: &str) -> Vec<XPathStep> {
    let mut steps = Vec::new();

    let parts: Vec<&str> = expression.split('/').filter(|s| !s.is_empty()).collect();

    for part in parts {
        let (axis, remainder) = if let Some(index) = part.find("::") {
            (Some(part[..index].to_string()), &part[index + 2..])
        } else {
            (None, part)
        };

        let (node_test, predicates) = if let Some(index) = remainder.find('[') {
            (Some(remainder[..index].to_string()), &remainder[index..])
        } else {
            (Some(remainder.to_string()), "")
        };

        let mut predicates_vec = Vec::new();

        let mut predicate = predicates;
        while let Some(start) = predicate.find('[') {
            if let Some(end) = predicate.find(']') {
                predicates_vec.push(predicate[start + 1..end].to_string());
                predicate = &predicate[end + 1..];
            } else {
                break;
            }
        }

        steps.push(XPathStep {
            axis,
            node_test,
            predicates: predicates_vec,
        });
    }

    steps
}

fn evaluate_relative_xpath(tree_node: Graph<XmlNode>, relative_xpath: String) -> Option<Graph<XmlNode>> {
    log::trace!("In evaluate_relative_xpath");

    log::debug!("node: {}", read_lock!(tree_node).data.describe());
    log::debug!("node id: {}", read_lock!(tree_node).id);
    log::debug!("relative_xpath: {}", relative_xpath);

    let mut current_node: Graph<XmlNode> = Arc::clone(&tree_node);
    let mut found_target = false;

    let steps = parse_xpath(&relative_xpath);

    for (i, step) in steps.iter().enumerate() {
        log::debug!("Step {}: {:?}", i + 1, step);

        if let Some(axis) = &step.axis {
            found_target = false;

            if axis == "preceding-sibling" {
                let parent;
                {
                    let rl = read_lock!(current_node);
                    parent = rl.parents.get(0).unwrap().clone();
                }
                let parent_children = &read_lock!(parent).children;

                for i in 0..parent_children.len() {
                    if read_lock!(parent_children[i]).id == read_lock!(current_node).id {
                        let preceding_sibling = &parent_children[i - 1];

                        current_node = Arc::clone(&preceding_sibling);
                        found_target = true;
                        break;
                    }
                }
            }
        }


        if let Some(node_test) = &step.node_test {
            found_target = false;

            if node_test.starts_with('@') {

                let key_value = &node_test[1..];
                let parts: Vec<&str> = key_value.split('=').collect();

                if parts.get(0).is_none() || parts.get(1).is_none() {
                    continue;
                }

                let key = parts[0];
                let value = parts[1].trim_matches('\'');

                bft(Arc::clone(&current_node), &mut |node: Graph<XmlNode>| {

                    let xml = read_lock!(node).data.clone();
                    
                    if let Some(xml_value) = xml.get_attribute_value(key) {

                        if xml_value == value {
                            current_node = Arc::clone(&node);
                            found_target = true;
                            return false;
                        }
                    }

                    true
                });
            } else {
                // TODO
                found_target = true;
            }
        }
    }

    if found_target {
       return Some(Arc::clone(&current_node));
    }

    None
}

/*********************************************************************
*********************************************************************/

















fn process_node(
    output_node: Graph<XmlNode>,
    basis_graph: Graph<BasisNode>,
    content: &mut Content,
) {
    let output_node_xml: XmlNode = read_lock!(output_node).data.clone();
    let lineage = get_lineage(Arc::clone(&output_node));
    let basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), lineage);

    let data = read_lock!(basis_node).data.data.clone();
    for node_data in read_lock!(data).iter() {
        if let Some(text_data) = &node_data.text {
            if text_data.is_presentational {
                log::info!("Discarding presentational text node data");
                continue;
            }
        }

        if let Some(element_data) = &node_data.element {
            if element_data.attribute == "href" && !element_data.is_page_link {
                log::info!("Discarding href action link...");
                continue;
            }
        }

        let content_value = ContentValue {
            name: node_data.name.clone(),
            value: node_data.value(&output_node_xml),
            meta: ContentValueMetadata {
                is_primary_content: node_data.clone().text.map_or(false, |text| text.is_primary_content)
            },
        };

        content.values.push(content_value);
    }

    let structures = read_lock!(basis_node).data.structure.clone();
    for structure in read_lock!(structures).iter() {



        //if let Some(root_node_xpath) = structure.root_node_xpath.clone() {
        //    if let Some(root_node) = evaluate_relative_xpath(Arc::clone(&output_node), root_node_xpath) {

        //        let meta = ContentMetadataRecursive {
        //            is_root: true,
        //            parent_id: None,
        //        };

        //        content.meta.recursive = Some(meta);

        //    } else {
        //        let parent_node_xpath = structure.parent_node_xpath.clone().unwrap();

        //        if let Some(parent_node) = evaluate_relative_xpath(Arc::clone(&output_node), parent_node_xpath) {

        //            let meta = ContentMetadataRecursive {
        //                is_root: false,
        //                parent_id: Some(read_lock!(parent_node).id.clone()),
        //            };

        //            content.meta.recursive = Some(meta);

        //        }
        //    }

        //}



    }
}

impl Traversal {
    pub fn from_tree(output_tree: Graph<XmlNode>) -> Self {
        Traversal {
            output_tree: output_tree,
            basis_graph: None,
        }
    }

    pub fn with_basis(mut self, graph: Graph<BasisNode>) -> Self {
        self.basis_graph = Some(Arc::clone(&graph));

        self
    }

    pub fn harvest(self) -> Result<String, Errors> {
        let mut content = Content {
            id: read_lock!(self.output_tree).id.clone(),
            meta: ContentMetadata {
                recursive: None,
            },
            values: Vec::new(),
            children: Vec::new(),
        };

        fn recurse(
            mut output_node: Graph<XmlNode>,
            basis_graph: Graph<BasisNode>,
            output_content: &mut Content,
        ) {
            if read_lock!(output_node).is_linear_tail() {
                panic!("Did not expect to encounter node in linear tail");
            }

            if read_lock!(output_node).is_linear_head() {
                log::info!("Output node is head of linear sequence of nodes");

                while read_lock!(output_node).is_linear() {
                    process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);

                    output_node = {
                        let next_node = read_lock!(output_node).children.first().expect("Linear output node has no children").clone();
                        next_node.clone()
                    };
                }

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);
            } else {
                log::info!("Output node is non-linear");

                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content);
            }

            for child in read_lock!(output_node).children.iter() {
                let mut child_content = Content {
                    id: read_lock!(child).id.clone(),
                    meta: ContentMetadata {
                        recursive: None,
                    },
                    values: Vec::new(),
                    children: Vec::new(),
                };

                recurse(
                    Arc::clone(child),
                    Arc::clone(&basis_graph),
                    &mut child_content,
                );

                output_content.children.push(child_content);
            }
        }

        recurse(
            Arc::clone(&self.output_tree),
            Arc::clone(&self.basis_graph.unwrap()),
            &mut content,
        );

        log::info!("Removing empty objects from content...");
        content.remove_empty();

        let output = Output {
            data: content,
        };

        let output_format = DEFAULT_OUTPUT_FORMAT;
        log::debug!("output_format: {:?}", output_format);

        match output_format {
            OutputFormats::JSON => {
                log::info!("Harvesting tree as JSON");

                let serialized = serde_json::to_string(&output).expect("Could not serialize output to JSON");

                Ok(serialized)
            },
        }
    }
}
