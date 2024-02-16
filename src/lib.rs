extern crate simple_logging;
extern crate log;

use serde::{Serialize, Value};
use tokio::runtime::Runtime;
use std::fs::{OpenOptions, File};
use std::process;
use std::io::{Read};
use std::io::{self};

mod models {
    pub mod chat;
    pub mod list;
}
mod parsers {
    pub mod chat;
    pub mod list;
}
mod transformers {
    pub mod chat;
    pub mod list;
}

#[derive(Clone)]
enum Errors {
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
enum Document {
    Chat(models::chat::Chat),
    List(models::list::List),
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Serialize)]
enum Parser {
    Chat(models::chat::ChatParser),
    List(models::list::ListParser),
}

#[derive(Debug, Serialize)]
struct Output {
    parsers: Vec<Parser>,
    data: Vec<Document>,
}

pub fn string_to_json(document: String, document_type: &str) -> Result<Output, Errors> {
    log::trace!("In string_to_json");
    log::debug!("document_type: {}", document_type);

    if document.trim().is_empty() {
        log::info!("Document not provided, aborting...");
        return Err(Errors::DocumentNotProvided);
    }

    let parsers = get_parsers(document, document_type)?;

    let mut output = Output {
        parsers: parsers.clone(),
        data: Vec::new(),
    };

    for parser in parsers.iter() {
        let result = parse_document(document, document_type, &parser)?;
        output.data.push(result);
    }
    
    return Ok(output);
}

pub fn file_to_json(file_name: String, document_type: &str) -> Result<Output, Errors> {
    log::trace!("In file_to_json");
    log::debug!("file_name: {}", file_name);
    log::debug!("document_type: {}", document_type);

    let mut document = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut document).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    return string_to_json(document, document_type);
}

pub fn parse_document(document: String, document_type: &str, parser: Value) -> Result<Document, Errors> {
    log::trace!("In parse_text");
    log::debug!("document_type: {}", document_type);

    match document_type {
        "chat" => {
            let chat = transformers::chat::transform_document_to_chat(document.clone(), &parser);
            Ok(Document::Chat(chat))
        }
        "list" => {
            let list = transformers::list::transform_document_to_list(document.clone(), &parser);
            Ok(Document::List(list))
        }
        _ => {
            Err(Errors::UnexpectedDocumentType)
        }
    }
}

pub fn get_parsers(document: String, document_type: &str) -> Result<Vec<Parser>, Errors> {
    log::trace!("In get_parsers");
    log::debug!("document_type: {}", document_type);

    let chunks = chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        match document_type {
            "chat" => {
                let chat_parsers = parsers::chat::get_chat_parser(sample).await;
                if let Ok(ok_chat_parsers) = chat_parsers {

                    let parsers: Vec<Parser> = ok_chat_parsers
                        .iter()
                        .map(|parser| {
                            Parser::Chat(parser.clone())
                        })
                        .collect();

                    Ok(parsers)
                } else {
                    Err(Errors::UnexpectedError)
                }
            }
            "list" => {
                let list_parsers = parsers::list::get_list_parser(sample).await;
                if let Ok(ok_list_parsers) = list_parsers {

                    let parsers: Vec<Parser> = ok_list_parsers
                        .iter()
                        .map(|parser| {
                            Parser::List(parser.clone())
                        })
                        .collect();

                    Ok(parsers)
                } else {
                    Err(Errors::UnexpectedError)
                }
            }
            _ => {
                Err(Errors::UnexpectedDocumentType)
            }
        }
    });

    return Err(Errors::UnexpectedError);
}

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    log::trace!("In chunk_string");

    s.chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
