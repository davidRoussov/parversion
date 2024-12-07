use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::collections::{HashMap, HashSet};

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData, bft};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::macros::*;
use crate::basis_graph::{BasisGraph};
use crate::node_data_structure::{apply_structure};
use crate::node_data::{apply_data};
use crate::content::{
    Content,
    ContentMetadataAssociative,
    postprocess_content
};
use crate::error::{Errors};

#[derive(Debug)]
pub enum HarvestFormats {
    JSON,
    XML,
}

impl FromStr for HarvestFormats {
    type Err = String;

    fn from_str(input: &str) -> Result<HarvestFormats, Self::Err> {
        match input.to_lowercase().as_str() {
            "json" => Ok(HarvestFormats::JSON),
            "xml" => Ok(HarvestFormats::XML),
            _ => Err(format!("'{}' is not a valid format", input)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Harvest {
    pub content: Content,
    pub related_content: Content,
}

pub fn serialize_harvest(harvest: Harvest, format: HarvestFormats) -> Result<String, Errors> {
    match format {
        HarvestFormats::JSON => {
            log::info!("Serializing harvest as JSON");

            let serialized = serde_json::to_string(&harvest)
                .expect("Could not serialize output to JSON");

            Ok(serialized)
        },
        HarvestFormats::XML => {
            log::info!("Serializing harvest as XML");
            unimplemented!()
        }
    }
}

pub fn harvest(
    output_tree: Graph<XmlNode>,
    basis_graphs: Vec<BasisGraph>,
) -> Harvest {
    log::trace!("In harvest");

    let mut visited: HashSet<String> = HashSet::new();

    let mut content = Content::default();
    content.id = read_lock!(output_tree).id.clone();
    let mut related_content = Content::default();
    related_content.id = read_lock!(output_tree).id.clone();

    fn recurse(
        mut output_node: Graph<XmlNode>,
        basis_graphs: Vec<BasisGraph>,
        output_content: &mut Content,
        output_related_content: &mut Content,
        visited: &mut HashSet<String>,
    ) {

        if visited.contains(&read_lock!(output_node).id) {
            return;
        } else {
            visited.insert(read_lock!(output_node).id.clone());
        }

        if read_lock!(output_node).is_linear() {
            log::info!("Output node is linear");

            while read_lock!(output_node).is_linear() {
                process_node(
                    Arc::clone(&output_node),
                    basis_graphs.clone(),
                    output_content,
                    output_related_content
                );

                output_node = {
                    let next_node = read_lock!(output_node).children.first().expect("Linear output node has no children").clone();
                    next_node.clone()
                };
            }

            process_node(
                Arc::clone(&output_node),
                basis_graphs.clone(),
                output_content,
                output_related_content
            );
        } else {
            log::info!("Output node is non-linear");

            process_node(
                Arc::clone(&output_node),
                basis_graphs.clone(),
                output_content,
                output_related_content
            );
        }

        for child in read_lock!(output_node).children.iter() {
            let mut child_content = Content::default();
            child_content.id = read_lock!(child).id.clone();
            child_content.lineage = read_lock!(child).lineage.clone();
            let mut child_related_content = Content::default();
            child_related_content.id = read_lock!(child).id.clone();
            child_related_content.lineage = read_lock!(child).lineage.clone();

            recurse(
                Arc::clone(child),
                basis_graphs.clone(),
                &mut child_content,
                &mut child_related_content,
                visited,
            );

            output_content.inner_content.push(child_content);
            output_related_content.inner_content.push(child_related_content);
        }
    }

    recurse(
        Arc::clone(&output_tree),
        basis_graphs.clone(),
        &mut content,
        &mut related_content,
        &mut visited
    );

    postprocess_content(&mut content);
    postprocess_content(&mut related_content);

    Harvest {
        content: content,
        related_content: related_content,
    }
}

fn find_basis_node(
    output_node: Graph<XmlNode>,
    basis_graphs: Vec<BasisGraph>,
) -> Option<(BasisGraph, Graph<BasisNode>)> {
    log::trace!("In find_basis_node");

    let mut target: Option<(BasisGraph, Graph<BasisNode>)> = None;

    for (index, basis_graph) in basis_graphs.iter().enumerate() {
        log::info!("Searching for node in graph #{}/{}", index + 1, basis_graphs.len());

        bft(Arc::clone(&basis_graph.root), &mut |basis_node: Graph<BasisNode>| {
            if read_lock!(basis_node).lineage == read_lock!(output_node).lineage {
                target = Some((basis_graph.clone(), Arc::clone(&basis_node)));
                return false;
            }

            true
        });

        if target.is_some() {
            break;
        }
    }

    target
}

fn process_node(
    output_node: Graph<XmlNode>,
    basis_graphs: Vec<BasisGraph>,
    content: &mut Content,
    related_content: &mut Content,
) {
    log::trace!("In process_node");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
PROCESSING OUTPUT NODE:
{}
Node:       {}
Hash:       {}
Lineage:    {}
{}",
            block_separator,
            block_separator,
            read_lock!(output_node).data.describe(),
            read_lock!(output_node).hash,
            read_lock!(output_node).lineage,
            block_separator,
        ));
    }

    if let Some((basis_graph, basis_node)) = find_basis_node(
        Arc::clone(&output_node),
        basis_graphs.clone()
    ) {
        log::info!("Found basis node");

        let data = read_lock!(basis_node).data.data.clone();
        for node_data in read_lock!(data).iter() {
            log::debug!("node_data: {:?}", node_data);

            if let Some(content_value) = apply_data(node_data.clone(), Arc::clone(&output_node)) {
                let is_peripheral = {
                    node_data.clone().text.map_or(false, |text| text.is_peripheral_content) ||
                    node_data.clone().element.map_or(false, |element| element.is_peripheral_content)
                };
                log::debug!("is_peripheral: {}", is_peripheral);

                if is_peripheral {
                    related_content.values.push(content_value);
                } else {
                    content.values.push(content_value);
                }
            }
        }

        let structures = read_lock!(basis_node).data.structure.clone();
        for structure in read_lock!(structures).iter() {
            log::debug!("structure: {:?}", structure);

            if let Some(associative) = structure.associative.clone() {
                log::debug!("Found an associative structure");

                let subgraph_hash = read_lock!(output_node).hash.clone();
                let mut associated_subgraphs = Vec::new();

                for group in associative.subgraph_ids {
                    if group.contains(&subgraph_hash) {
                        let filtered: Vec<String> = group
                            .into_iter()
                            .filter(|s| s != &subgraph_hash)
                            .collect();

                        associated_subgraphs.extend(filtered);
                    }
                }

                content.meta.associative = Some(ContentMetadataAssociative {
                    subgraph: subgraph_hash,
                    associated_subgraphs: associated_subgraphs,
                });
            } else {
                let meta = apply_structure(
                    structure.clone(),
                    Arc::clone(&output_node),
                    Arc::clone(&basis_graph.root),
                );

                if let Some(recursive) = meta.recursive {
                    content.meta.recursive = Some(recursive.clone());
                    related_content.meta.recursive = Some(recursive.clone());
                }

                if let Some(enumerative) = meta.enumerative {
                    content.meta.enumerative = Some(enumerative.clone());
                    related_content.meta.enumerative = Some(enumerative.clone());
                }
            }
        }

        log::info!("Done applying basis node to output node");
    } else {
        log::warn!("Could not find basis node using output node lineage");
    }
}
