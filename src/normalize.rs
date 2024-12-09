use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use serde_json::{Value};

use crate::basis_graph::BasisGraph;

pub struct Normalization {
    pub basis_graph: BasisGraph,
    pub related_data: OutputData,
    pub normalized_data: OutputData,
}

pub fn normalize_file(
    file_name: String,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file_name: {}", file_name);

    let mut text = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut text).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    normalize_text(text, options)
}

pub fn normalize_text(
    text: String,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, options)?;

    normalize_document(document, options)
}

pub async fn normalize_document(
    document: Document,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document");

    let organization = organization::organize_document(document, options);

    normalize_organization(organization, options)
}

pub async fn normalize_organization(
    organization: Organization,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_organization");

    let Organization {
        basis_graph,
        organized_data,
        related_data
    } = organization;

    let normalized_data = basis_graph.normalize(organized_data).await;

    Normalization {
        basis_graph,
        related_data,
        normalized_data,
    }
}
