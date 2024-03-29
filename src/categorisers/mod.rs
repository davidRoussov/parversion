use serde_json::{Value};

use crate::models;
use crate::prompts;
use crate::utilities;

pub enum Errors {
    LlmRequestError,
    LlmInvalidRegex,
}

pub async fn get_document_types(document: String) -> Result<Vec<models::document_type::DocumentType>, Errors> {
    log::trace!("In get_document_type");

    let mut document_types: Vec<models::document_type::DocumentType> = Vec::new();

    let chunks = utilities::text::chunk_string(&document, 20000);
    log::debug!("number of chunks: {}", chunks.len());

    let sample = &chunks[0];

    let prompt = format!("{} {}", prompts::document_types::DOCUMENT_TYPES_PROMPT, sample);
    let llm_response = utilities::llm::get_llm_response(prompt).await;
    match llm_response {
        Ok(value) => {
            log::info!("Successfully obtained response from LLM");
            log::debug!("value: {:?}", value);

            let json = value.as_object().unwrap();

            for (key, value) in json.iter() {
                log::debug!("key: {}", key);

                if let Value::Object(sub_object) = value {
                    if let Some(Value::Bool(is_present)) = sub_object.get("is_present") {
                        if *is_present {
                            if key == "article" {
                                document_types.push(models::document_type::DocumentType::Article);
                            } else if key == "long_form" {
                                document_types.push(models::document_type::DocumentType::LongForm);
                            } else if key == "chat" {
                                document_types.push(models::document_type::DocumentType::Chat);
                            } else if key == "weather" {
                                document_types.push(models::document_type::DocumentType::Weather);
                            } else if key == "business_details" {
                                document_types.push(models::document_type::DocumentType::BusinessDetails);
                            } else if key == "curated_listing" {
                                document_types.push(models::document_type::DocumentType::CuratedListing);
                            } else if key == "event_listing" {
                                document_types.push(models::document_type::DocumentType::EventListing);
                            } else if key == "job_listing" {
                                document_types.push(models::document_type::DocumentType::JobListing);
                            } else if key == "real_estate_listing" {
                                document_types.push(models::document_type::DocumentType::RealEstateListing);
                            } else if key == "search_engine_listing" {
                                document_types.push(models::document_type::DocumentType::SearchEngineListing);
                            }

                        }
                    }
                }
            }
        }
        Err(error) => {
            log::error!("{}", error);
            return Err(Errors::LlmRequestError);
        }
    } 

    Ok(document_types)
}
