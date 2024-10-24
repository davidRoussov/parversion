use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData, graph_hash};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::macros::*;
use crate::basis_graph::{BasisGraph};
use crate::node_data_structure::{apply_structure};
use crate::node_data::{apply_data};
use crate::content::{
    Content,
    ContentMetadata,
    ContentMetadataAssociative,
    postprocess_content
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Harvest {
    pub content: Content,
    pub related_content: Content,
}

fn process_node(
    output_node: Graph<XmlNode>,
    basis_graph: Graph<BasisNode>,
    content: &mut Content,
    related_content: &mut Content,
) {
    log::trace!("In process_node");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
PROCESSING NODE:
{}
Node:   {}
{}",
            block_separator,
            block_separator,
            read_lock!(output_node).data.describe(),
            block_separator,
        ));
    }


    let lineage = get_lineage(Arc::clone(&output_node));

    if let Some(basis_node) = apply_lineage(Arc::clone(&basis_graph), lineage) {




        // If a node has a parent which is an element with an href that is interpreted to be an "action" link, we discard it
        // These nodes only describe the action link
        // e.g. <a href="reply?id=123">reply</a>
        let rl = read_lock!(output_node);
        if let Some(output_node_parent) = rl.parents.get(0) {
            let output_node_parent_lineage = get_lineage(Arc::clone(&output_node_parent));
            let output_node_parent_basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), output_node_parent_lineage).unwrap();
            let output_node_parent_basis_node_data = read_lock!(output_node_parent_basis_node).data.data.clone();
            let is_parent_action = read_lock!(output_node_parent_basis_node_data).iter().any(|item| {
                if let Some(element_data) = &item.element {
                    return element_data.attribute == "href" && !element_data.is_page_link;
                }

                false
            });

            if is_parent_action {
                log::info!("Discarding node data whose parent is an action href");
                return;
            }
        }


        




        let data = read_lock!(basis_node).data.data.clone();
        for node_data in read_lock!(data).iter() {
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
            if let Some(associative) = structure.associative.clone() {


                let subgraph_hash = graph_hash(Arc::clone(&output_node));

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


                if !associated_subgraphs.is_empty() {
                    content.meta.associative = Some(ContentMetadataAssociative {
                        subgraph: subgraph_hash,
                        associated_subgraphs: associated_subgraphs,
                    });
                }





            } else {
                let meta = apply_structure(
                    structure.clone(),
                    Arc::clone(&output_node),
                    Arc::clone(&basis_graph),
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
    }
}

pub fn harvest(
    output_tree: Graph<XmlNode>,
    basis_graph: BasisGraph
) -> Harvest {
    log::trace!("In harvest");

    let mut content = Content::default();
    content.id = read_lock!(output_tree).id.clone();
    let mut related_content = Content::default();
    related_content.id = read_lock!(output_tree).id.clone();

    fn recurse(
        mut output_node: Graph<XmlNode>,
        basis_graph: Graph<BasisNode>,
        output_content: &mut Content,
        output_related_content: &mut Content,
    ) {
        if read_lock!(output_node).is_linear() {
            log::info!("Output node is linear");

            while read_lock!(output_node).is_linear() {
                process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);

                output_node = {
                    let next_node = read_lock!(output_node).children.first().expect("Linear output node has no children").clone();
                    next_node.clone()
                };
            }

            process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);
        } else {
            log::info!("Output node is non-linear");

            process_node(Arc::clone(&output_node), Arc::clone(&basis_graph), output_content, output_related_content);
        }

        for child in read_lock!(output_node).children.iter() {
            let mut child_content = Content::default();
            child_content.id = read_lock!(child).id.clone();
            let mut child_related_content = Content::default();
            child_related_content.id = read_lock!(child).id.clone();

            recurse(
                Arc::clone(child),
                Arc::clone(&basis_graph),
                &mut child_content,
                &mut child_related_content,
            );

            output_content.inner_content.push(child_content);
            output_related_content.inner_content.push(child_related_content);
        }
    }

    recurse(
        Arc::clone(&output_tree),
        Arc::clone(&basis_graph.root),
        &mut content,
        &mut related_content,
    );

    postprocess_content(&mut content);
    postprocess_content(&mut related_content);

    Harvest {
        content: content,
        related_content: related_content,
    }
}
