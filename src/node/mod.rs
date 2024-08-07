use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tokio::time::{sleep, Duration};

use std::rc::{Rc};
use std::cell::RefCell;
use std::collections::{VecDeque, HashMap, HashSet};

mod debug;
mod interpretation;
mod traversal;
mod utility;

use crate::node_data::{NodeData};
use crate::node::traversal::*;
use crate::xml_node::*;
use crate::constants;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TreeMetadata {
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tree {
    pub root: Rc<Node>, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    pub id: String,
    pub hash: String,
    pub xml: XmlNode,
    pub parent: RefCell<Option<Rc<Node>>>,
    pub data: RefCell<Vec<NodeData>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}

impl Node {
    pub fn from_void() -> Rc<Self> {
        Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: constants::ROOT_NODE_HASH.to_string(),
            xml: XmlNode::from_void(),
            parent: None.into(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
        })
    }

    pub fn from_xml(xml: &XmlNode, parent: Option<Rc<Node>>) -> Rc<Self> {
        let node = Rc::new(Node {
            id: Uuid::new_v4().to_string(),
            hash: xml_to_hash(&xml),
            xml: xml.without_children(),
            parent: parent.into(),
            data: RefCell::new(Vec::new()),
            children: RefCell::new(vec![]),
        });

        let children: Vec<Rc<Node>> = xml.get_children().iter().map(|child| {
            Node::from_xml(child, Some(Rc::clone(&node)))
        }).collect();

        node.children.borrow_mut().extend(children);

        node
    }
}

pub fn deep_copy(node: &Rc<Node>) -> Rc<Node> {
    let new_node = Rc::new(Node {
        id: node.id.clone(),
        hash: node.hash.clone(),
        xml: node.xml.clone(),
        parent: RefCell::new(None),
        data: RefCell::new(node.data.borrow().clone()),
        children: RefCell::new(Vec::new()),
    });

    let children: Vec<Rc<Node>> = node.children.borrow().iter()
        .map(|child| {
            let child_copy = deep_copy(child);
            child_copy.parent.borrow_mut().replace(Rc::clone(&new_node));
            child_copy
        })
    .collect();
    new_node.children.replace(children);

    new_node
}

pub fn build_tree(xml: String) -> Rc<Node> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = XmlNode::parse(&mut reader).expect("Could not parse XML");

    Node::from_xml(&xml, None)
}

pub async fn get_tree_metadata(basis_tree: Rc<Node>) -> TreeMetadata {
    log::trace!("In get_tree_metadata");

    let db = sled::open("src/database/tree_metadata").expect("Could not connect to datbase");

    let title = basis_tree.get_tree_title(&db).await;

    TreeMetadata {
        title: title,
    }
}

pub async fn grow_tree(basis_tree: Rc<Node>, output_tree: Rc<Node>) {
    log::trace!("In grow_tree");

    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    let mut nodes: Vec<Rc<Node>> = Vec::new();

    post_order_traversal(basis_tree.clone(), &mut |node: &Rc<Node>| {
        nodes.push(node.clone());
    });

    log::info!("There are {} nodes to be evaluated", nodes.len());

    for (index, node) in nodes.iter().enumerate() {
        log::info!("--- Analysing node #{} out of {} ---", index + 1, nodes.len());
        log::debug!("id: {}, xml: {}", node.id, node.xml);

        let (node_data, should_sleep) = node.interpret_node(&db, &output_tree).await;

        if should_sleep {
            sleep(Duration::from_secs(1)).await;
        }



        // ***************




        let filtered_node_data = node_data.clone().into_iter().filter(|item| {
            if let Some(element_fields) = &item.element_fields {
                let attribute = &element_fields.attribute.as_str();
                let value = item.value(&node.xml);

                if constants::SEEN_BLACKLISTED_ATTRIBUTES.contains(attribute) {
                    log::warn!("Ignoring blacklisted attribute: {}", attribute);
                    return false;
                }


                // * href values that execute javascript

                if attribute == &"href" && value.trim_start().to_lowercase().starts_with("javascript") {
                    log::warn!("Ignoring href that executes javascript");
                    return false;
                }

                // * blank href values

                if attribute == &"href" && value.is_empty() {
                    log::warn!("Ignoring blank href");
                    return false;
                }

                // * anchor links

                if attribute == &"href" && value.starts_with("#") {
                    log::warn!("Ignoring anchor link");
                    return false;
                }


            }

            true
        }).collect();





        // ***************




        *node.data.borrow_mut() = filtered_node_data;
    }
}

pub async fn interpret(graph: Rc<Node>, output_tree: Rc<Node>) {
    log::trace!("In interpret");

    let db = sled::open("src/database/hash_to_node_data").expect("Could not connect to datbase");

    let mut nodes: Vec<Rc<Node>> = Vec::new();

    bfs_graph(Rc::clone(&graph), &mut |node: &Rc<Node>| {
        nodes.push(node.clone());
    });

    for (index, node) in nodes.iter().enumerate() {

        if node.xml.to_string() == "<tr class=\"athing\" id=\"40840396\" />" {
            log::info!("{}", "=".repeat(60));
            log::info!("Analyzing node #{}", index + 1);
            log::info!("{}", "=".repeat(60));

            let (node_data_structure, should_sleep) = node.interpret_node_structure(&db, &output_tree).await;

            if should_sleep {
                sleep(Duration::from_secs(1)).await;
            }

        }
    }
}

pub fn linearize(tree: Rc<Node>) {
    log::trace!("In linearize");

    fn dfs(
        node: Rc<Node>,
        visited_hashes: &mut HashMap<String, Rc<Node>>
    ) {
        let hash = node.hash.clone();

        if let Some(first_occurrence) = visited_hashes.get(&hash) {
            log::info!("Detected cycle");

            if let Some(parent) = node.parent.borrow().as_ref() {
                parent.children.borrow_mut()
                    .retain(|child| child.id != node.id);
                parent.children.borrow_mut().push(first_occurrence.clone());
            }

            let children = node.children.borrow_mut().drain(..).collect::<Vec<_>>();
            for child in children {
                *child.parent.borrow_mut() = Some(first_occurrence.clone()).into();
                first_occurrence.children.borrow_mut().push(child);
            }
        } else {
            visited_hashes.insert(hash.clone(), Rc::clone(&node));

            let children = node.children.borrow().clone();
            for child in children {
                dfs(
                    child,
                    visited_hashes,
                );
            }

            visited_hashes.remove(&hash);
        }
    }

    dfs(
        tree,
        &mut HashMap::new(),
    );
}

pub fn prune(tree: Rc<Node>) {
    log::trace!("In prune");

    bfs_graph(Rc::clone(&tree), &mut |node: &Rc<Node>| {
        loop {
            let children_borrow = node.children.borrow();
            log::debug!("Node has {} children", children_borrow.len());

            let purported_twins: Option<(Rc<Node>, Rc<Node>)> = children_borrow.iter()
                .find_map(|child| {
                    children_borrow.iter()
                        .find(|&sibling| sibling.id != child.id && sibling.hash == child.hash && sibling.parent.borrow().is_some())
                        .map(|sibling| (Rc::clone(child), Rc::clone(sibling)))
                });

            drop(children_borrow);

            if let Some(twins) = purported_twins {
                log::info!("Found two sibling nodes with the same hash: {}", twins.0.hash);
                log::info!("Pruning nodes with ids: {} and {} with hash {}", twins.0.id, twins.1.id, twins.0.hash);

                merge_nodes(node.clone(), twins);
            } else {
                break;
            }
        }
    });
}

pub fn absorb(recipient: Rc<Node>, donor: Rc<Node>) {
    log::trace!("In absorb");

    let recipient_child = {
        recipient.children.borrow().iter().find(|item| item.hash == donor.hash).cloned()
    };

    if let Some(recipient_child) = recipient_child {
        log::trace!("Donor and recipient node have the same hash");

        if recipient_child.subtree_hash() == donor.subtree_hash() {
            log::trace!("Donor and recipient child subtree hashes match");
            return;
        } else {
            log::trace!("Donor and recipient child have differing subtree hashes");
            let donor_children = donor.children.borrow().clone();

            for donor_child in donor_children.iter() {
                absorb(recipient_child.clone(), donor_child.clone());
            }
        }
    } else {
        log::trace!("Donor and recipient subtrees incompatible. Adopting donor node...");

        *donor.parent.borrow_mut() = Some(recipient.clone());
        recipient.children.borrow_mut().push(donor);
    }
}

pub fn find_all_node_xml_by_lineage(
    root: Rc<Node>,
    lineage: VecDeque<String>,
) -> Vec<String> {
    log::trace!("In find_all_node_xml_by_lineage");

    let mut target_xml = Vec::new();

    let mut queue = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    
    queue.push_back(root.clone());

    while let Some(current) = queue.pop_front() {
        let current_lineage = current.get_lineage();

        if current_lineage == lineage {
            target_xml.push(current.id.clone());
        } else if is_queue_prefix(&current_lineage, &lineage) {
            for child in current.children.borrow().iter() {
                if !visited.contains(&child.id) {
                    queue.push_back(child.clone());
                    visited.insert(child.id.clone());
                }
            }
        }
    }

    target_xml
}

fn is_queue_prefix(needle: &VecDeque<String>, haystack: &VecDeque<String>) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }

    for (n, h) in needle.iter().zip(haystack.iter()) {
        if n != h {
            return false;
        }
    }

    true
}

pub fn search_basis_tree_by_lineage(mut tree: Rc<Node>, mut lineage: VecDeque<String>) -> Option<Rc<Node>> {
    log::trace!("In search_basis_tree_by_lineage");

    while let Some(hash) = lineage.pop_front() {
        let node = tree
            .children
            .borrow()
            .iter()
            .find(|item| item.hash == hash)
            .cloned();

        if let Some(node) = node {
            tree = node;
        } else {
            log::info!("Could not find child node with hash");

            return None;
        }
    }

    Some(tree)
}

pub fn node_to_html_with_target_node(
    node: &Rc<Node>,
    target_node: Rc<Node>
) -> (
String, // html before target node
String, // target node opening tag
String, // target node child content
String, // target node closing tag
String, // html after target node
) {
    log::trace!("In node_to_html_with_target_node");

    let mut before_html = String::new();
    let mut target_opening_html = String::new();
    let mut target_child_content = String::new();
    let mut target_closing_html = String::new();
    let mut after_html = String::new();
    let mut found_target = false;

    fn recurse(
        current: Rc<Node>,
        target: Rc<Node>,
        found_target: &mut bool,
        before_html: &mut String,
        target_opening_html: &mut String,
        target_child_content: &mut String,
        target_closing_html: &mut String,
        after_html: &mut String
    ) {
        if let Some(element) = &current.xml.element {
            let opening_tag = get_opening_tag(&element);
            let closing_tag = get_closing_tag(&element);

            if *found_target {
                after_html.push_str(&opening_tag);
            } else if current.id == target.id {
                *found_target = true;
                target_opening_html.push_str(&opening_tag);
            } else {
                before_html.push_str(&opening_tag);
            }

            for child in current.children.borrow().iter() {
                recurse(
                    child.clone(),
                    target.clone(),
                    found_target,
                    before_html,
                    target_opening_html,
                    target_child_content,
                    target_closing_html,
                    after_html
                );
            }

            if *found_target && current.id == target.id {
                target_closing_html.push_str(&closing_tag);
            } else if *found_target {
                after_html.push_str(&closing_tag);
            } else {
                before_html.push_str(&closing_tag);
            }
        }

        if let Some(text) = &current.xml.text {
            if *found_target {
                after_html.push_str(&text.clone());
            } else if current.id == target.id {
                *found_target = true;
                target_child_content.push_str(&text.clone());
            } else {
                before_html.push_str(&text.clone());
            }
        }
    }

    recurse(
        node.clone(),
        target_node.clone(),
        &mut found_target,
        &mut before_html,
        &mut target_opening_html,
        &mut target_child_content,
        &mut target_closing_html,
        &mut after_html
    );

    (
        before_html,
        target_opening_html,
        target_child_content,
        target_closing_html,
        after_html
    )
}

pub fn find_node_by_id(root: &Rc<Node>, id: &str) -> Option<Rc<Node>> {
    if root.id == id {
        return Some(Rc::clone(root));
    }

    for child in root.children.borrow().iter() {
        if let Some(found) = find_node_by_id(child, id) {
            return Some(found);
        }
    }

    None
}

fn merge_nodes(parent: Rc<Node>, nodes: (Rc<Node>, Rc<Node>)) {
    log::trace!("In merge_nodes");

    *nodes.1.parent.borrow_mut() = None;

    for child in nodes.1.children.borrow_mut().iter() {
        *child.parent.borrow_mut() = Some(nodes.0.clone()).into();
        nodes.0.children.borrow_mut().push(child.clone());
    }

    parent.children.borrow_mut().retain(|child| child.id != nodes.1.id);
}
