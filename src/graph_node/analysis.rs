use tokio::sync::{OwnedSemaphorePermit};
use std::sync::{Arc};
use std::collections::HashSet;

use super::{
    Graph, 
    GraphNodeData, 
    find_homologous_nodes,
    build_xml_with_target_node,
    apply_lineage,
    get_lineage,
    bft,
    graph_hash
};
use crate::xml_node::{XmlNode, get_meaningful_attributes};
use crate::basis_node::{BasisNode};
use crate::node_data_structure::{NodeDataStructure, EnumerativeStructure};
use crate::macros::*;
use crate::config::{CONFIG};
use crate::constants;
use crate::llm::{interpret_data_structure, interpret_element_data, interpret_text_data};
use crate::harvest::{harvest, Harvest};
use crate::basis_graph::BasisGraph;
use crate::{serialize, HarvestFormats};

pub async fn analyze(
    target_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze");

    {
        let block_separator = "=".repeat(60);
        log::info!("{}", format!(
        "\n{}
ANALYZING NODE:
{}
Node:   {}
Hash:   {}
{}",
            block_separator,
            block_separator,
            read_lock!(target_node).data.describe(),
            read_lock!(target_node).hash,
            block_separator,
        ));
    }

    {
        // When a basis graph has already been populated with interpreted nodes on a previous
        // iteration, we are obviously unlikely to find this node again in the current output tree
        // For now, let's check if the basis node has data on it and ignore such nodes
        let basis_node = &read_lock!(target_node).data;

        if !read_lock!(basis_node.data).is_empty() || !read_lock!(basis_node.structure).is_empty() {
            log::info!("Basis node has already been interpreted, not proceeding any further.");
            return;
        }
    }

    let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
        Arc::clone(&target_node),
        Arc::clone(&basis_root_node),
        Arc::clone(&output_tree),
    );

    if analyze_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node analyzed classically completely, not proceeding any further...");
        return;
    }

    analyze_structure(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
    analyze_data(
        Arc::clone(&target_node),
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
    ).await;
}

pub async fn analyze_associations(
    basis_node: Graph<BasisNode>,
    basis_root_node: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
    _permit: OwnedSemaphorePermit
) {
    log::trace!("In analyze_associations");

    //if analyze_associations_classically(
    //    Arc::clone(&basis_node),
    //    Arc::clone(&basis_root_node),
    //    Arc::clone(&output_tree)
    //) {
    //    log::info!("Basis node associations determined classically, not proceeding any further");
    //    return;
    //}






    log::debug!("basis node: {}", read_lock!(basis_node).data.describe());

    {
        let binding = read_lock!(basis_node);

        // TODO: what if more than one parent?
        if binding.parents.len() == 1 {
            let target_node_parent: Graph<BasisNode> = binding.parents.first().unwrap().clone();

            let mut basis_node_siblings: Vec<Graph<BasisNode>> = read_lock!(target_node_parent)
                .children
                .iter()
                .filter(|child| {
                    read_lock!(child).id != binding.id
                })
                .cloned()
                .collect();

            if basis_node_siblings.is_empty() {
                log::info!("Basis node does not have siblings");
                return;
            }

            basis_node_siblings.push(Arc::clone(&basis_node));


            log::info!("Going to infer sibling associations for basis node: {}", binding.data.describe());

            let mut harvests: Vec<Harvest> = Vec::new();

            for sibling in basis_node_siblings.iter() {
                log::debug!("sibling: {}", read_lock!(sibling).data.describe());

                let homologous_nodes: Vec<Graph<XmlNode>> = find_homologous_nodes(
                    Arc::clone(&sibling),
                    Arc::clone(&basis_root_node),
                    Arc::clone(&output_tree),
                );

                let mut unique_hashes = HashSet::new();

                let exemplary_nodes: Vec<Graph<XmlNode>> = homologous_nodes
                    .into_iter()
                    .filter(|node| {
                        let hash = graph_hash(Arc::clone(&node));
                        unique_hashes.insert(hash)
                    })
                    .collect();

                for exemplary_node in exemplary_nodes.iter() {
                    let basis_graph = BasisGraph {
                        root: Arc::clone(&basis_root_node),
                        subgraph_hashes: vec![],
                    };

                
                    let harvest = harvest(Arc::clone(&exemplary_node), basis_graph.clone());
                    
                    harvests.push(harvest);
                }

            }


            fn truncate(s: &str) -> &str {
                s.char_indices().nth(2000).map_or(s, |(idx, _)| &s[..idx])
            }



            let mut harvests: Vec<Harvest> = harvests.iter().cloned().filter(|item| {
                !(item.content.values.is_empty() && item.content.inner_content.is_empty())
            }).collect();

            if harvests.len() > 1 {

                log::debug!("basis_node: {}", binding.data.describe());
                for harvest in harvests.iter() {
                    let serialized = serialize(harvest.clone(), HarvestFormats::JSON).expect("Unable to serialize result");
                    log::debug!("harvest: {}", truncate(&serialized));
                }

            }









        }
    }






    
}


fn analyze_classically(target_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_classically");

    // * Basis root node
    if read_lock!(target_node).hash == constants::ROOT_NODE_HASH {
        log::info!("Node is root node, probably don't need to do anything here");
        return true;
    } else {
        if homologous_nodes.is_empty() {
            log::warn!("There cannot be zero homologous nodes for any basis node with respect to output tree.");
            return true;
        }
    }

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    // * Link elements
    if read_lock!(output_node).data.get_element_tag_name() == "link" {
        log::info!("Node represents HTML link element. Not proceeding any further.");
        return true;
    }

    // * Meta elements
    if read_lock!(output_node).data.get_element_tag_name() == "meta" {
        log::info!("Node represents HTML meta element. Not proceeding any further.");
        return true;
    }

    // * Script elements
    if read_lock!(output_node).data.get_element_tag_name() == "script" {
        log::info!("Node represents HTML script element. Not proceeding any further.");
        return true;
    }

    // * Head elements
    if read_lock!(output_node).data.get_element_tag_name() == "head" {
        log::info!("Node represents HTML head element. Not proceeding any further.");
        return true;
    }

    // * Body elements
    if read_lock!(output_node).data.get_element_tag_name() == "body" {
        log::info!("Node represents HTML body element. Not proceeding any further.");
        return true;
    }

    // * br elements
    if read_lock!(output_node).data.get_element_tag_name() == "br" {
        log::info!("Node represents HTML break element. Not proceeding any further.");
        return true;
    }

    // * form elements
    if read_lock!(output_node).data.get_element_tag_name() == "form" {
        log::info!("Node represents HTML form element. Not proceeding any further.");
        return true;
    }

    false
}

async fn analyze_structure(
    target_node: Graph<BasisNode>, 
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>
) {
    log::trace!("In analyze_structure");

    if analyze_structure_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node structure analyzed classically, not proceeding any further...");
        return;
    }

    let target_node_examples_count = read_lock!(CONFIG)
        .llm
        .data_structure_interpretation
        .target_node_examples_max_count
        .clone();
    let target_node_examples_count = std::cmp::min(
        target_node_examples_count,
        homologous_nodes.len()
    );
    let target_node_adjacent_xml_length = read_lock!(CONFIG)
        .llm
        .data_structure_interpretation
        .target_node_adjacent_xml_length
        .clone();
    let snippets = make_snippets(
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
        target_node_examples_count,
        target_node_adjacent_xml_length
    );

    let recursive_structure = interpret_data_structure(snippets).await;
    let node_data_structure = NodeDataStructure {
        recursive: Some(recursive_structure),
        enumerative: None,
        associative: None,
    };

    {
        let rl = read_lock!(target_node);
        let mut wl = write_lock!(rl.data.structure);
        wl.push(node_data_structure);
    }
}

async fn analyze_data(
    target_node: Graph<BasisNode>, 
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>
) {
    log::trace!("In analyze_data");

    if analyze_data_classically(Arc::clone(&target_node), homologous_nodes.clone()) {
        log::info!("Basis node data analyzed classically, not proceeding any further...");
        return;
    }

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    let target_node_examples_count = read_lock!(CONFIG).llm.target_node_examples_max_count.clone();
    let target_node_examples_count = std::cmp::min(target_node_examples_count, homologous_nodes.len());
    let target_node_adjacent_xml_length = read_lock!(CONFIG).llm.target_node_adjacent_xml_length;
    let snippets = make_snippets(
        homologous_nodes.clone(),
        Arc::clone(&output_tree),
        target_node_examples_count,
        target_node_adjacent_xml_length
    );

    if read_lock!(output_node).data.is_text() {
        let interpretation = interpret_text_data(snippets).await;

        {
            let rl = read_lock!(target_node);
            let mut wl = write_lock!(rl.data.data);
            wl.push(interpretation);
        }
    } else {

        let meaningful_attributes = get_meaningful_attributes(&read_lock!(output_node).data)
            .keys()
            .cloned()
            .collect();

        let interpretation = interpret_element_data(meaningful_attributes, snippets).await;

        {
            let rl = read_lock!(target_node);
            let mut wl = write_lock!(rl.data.data);
            wl.extend(interpretation);
        }
    }
}

fn analyze_data_classically(_basis_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_data_classically");

    let output_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();

    if read_lock!(output_node).data.is_element() {
        let meaningful_attributes = get_meaningful_attributes(&read_lock!(output_node).data);

        if meaningful_attributes.is_empty() {
            log::info!("Node represents HTML element without any meaningful attributes. Not proceeding any further.");

            return true;
        }
    }

    false
}

fn make_snippets(
    homologous_nodes: Vec<Graph<XmlNode>>,
    output_tree: Graph<XmlNode>,
    target_node_examples_count: usize,
    target_node_adjacent_xml_length: usize,
) -> Vec<String> {
    log::trace!("In make_snippets");
    log::info!("Using {} examples of target node for analysis", target_node_examples_count);
    
    let snippets: Vec<String> = homologous_nodes[..target_node_examples_count]
        .to_vec()
        .iter()
        .map(|item| node_to_snippet(Arc::clone(item), Arc::clone(&output_tree), target_node_adjacent_xml_length))
        .collect();

    snippets
}

fn analyze_structure_classically(basis_node: Graph<BasisNode>, homologous_nodes: Vec<Graph<XmlNode>>) -> bool {
    log::trace!("In analyze_structure_classically");

    let exemplary_node: Graph<XmlNode> = homologous_nodes.first().unwrap().clone();
    let output_parent_node: Option<Graph<XmlNode>> = read_lock!(exemplary_node).parents.first().cloned();

    if let Some(ref exemplary_parent) = output_parent_node {
        if homologous_nodes.len() > 1 {
            log::info!("Homologous node count is greater than one.");

            // Do all homologous nodes have the same parent?
            let are_siblings = homologous_nodes.iter().fold(true, |acc, node| {
                let parent = read_lock!(node).parents.first().cloned();
                let parent = parent.unwrap();

                acc && read_lock!(exemplary_parent).id == read_lock!(parent).id
            });
            log::debug!("are_siblings: {}", are_siblings);

            // If all homologous nodes have the same parent, that means this node represents a list of items of some kind
            if are_siblings {
                log::info!("Identified enumerative content");

                let enumerative_structure = EnumerativeStructure {
                    intrinsic_component_ids: vec![read_lock!(basis_node).id.clone()]
                };
                let node_data_structure = NodeDataStructure {
                    recursive: None,
                    enumerative: Some(enumerative_structure),
                    associative: None,
                };

                let binding = read_lock!(basis_node);
                let mut write_lock = write_lock!(binding.data.structure);
                write_lock.push(node_data_structure);
            }
        }
    }

    // Text nodes do not represent complex relationships
    if read_lock!(exemplary_node).data.is_text() {
        log::info!("Node is a text node. Not proceeding any further.");
        return true;
    }

    // Assuming nodes that are the lone child of their parent do not represent
    // any complex relationships to other nodes
    if let Some(ref parent) = output_parent_node {
        let parent_out_degree = read_lock!(parent).children.len();

        if parent_out_degree < 2 {
            log::info!("Node parent has out-degree less than two. Not proceeding any further.");
            return true;
        }
    } else {
        log::info!("Output node is root node. Not proceeding any further.");
        return true;
    }

    // If a basis node is part of a cycle, it represents a recursive relationship
    // in the underlying data model 
    let parent_count = read_lock!(basis_node).parents.len();
    if parent_count > 1 {
        log::info!("Node has more than one parent and is therefore recursive. Not proceeding any further.");
        return true;
    }

    false
}

fn node_to_snippet(
    node: Graph<XmlNode>,
    output_tree: Graph<XmlNode>,
    context_length: usize,
) -> String {
    log::trace!("In node_to_snippet");

    let document = build_xml_with_target_node(Arc::clone(&output_tree), Arc::clone(&node));

    if read_lock!(node).data.is_text() {
        format!(
            "{}<!--Target node start -->{}<!--Target node end -->{}",
            take_from_end(&document.0, context_length),
            document.2,
            take_from_start(&document.4, context_length),
        )
    } else {
        let after_start_tag = &format!(
            "{}{}{}",
            document.2,
            document.3,
            document.4
        );

        format!(
            "{}<!--Target node start -->{}<!--Target node end -->{}",
            take_from_end(&document.0, context_length),
            document.1,
            take_from_start(after_start_tag, context_length),
        )
    }
}

fn take_from_end(s: &str, amount: usize) -> &str {
    log::trace!("In take_from_end");

    let len = s.len();
    if amount >= len {
        s
    } else {
        let start_index = len - amount;
        let mut adjusted_start = start_index;

        while !s.is_char_boundary(adjusted_start) && adjusted_start < len {
            adjusted_start += 1;
        }

        &s[adjusted_start..]
    }
}

fn take_from_start(s: &str, amount: usize) -> &str {
    log::trace!("In take_from_end");

    if amount >= s.len() {
        s
    } else {
        let end_index = amount;
        let mut adjusted_end = end_index;

        while !s.is_char_boundary(adjusted_end) && adjusted_end > 0 {
            adjusted_end -= 1;
        }

        &s[..adjusted_end]
    }
}
