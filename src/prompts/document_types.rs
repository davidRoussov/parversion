pub static DOCUMENT_TYPES_PROMPT: &str = r##"
Hi ChatGPT. You job is to categorize documents based on their probable type. Review the following JSON to see the possible document types. The is_present key is the value you will need to modify; set it to true if the document contains content corresponding to its parent key, with the 'criteria' key providing more information about how to classify the document. Do not set is_present to true unless the document I provide you directly includes content of a particular type. Ignore urls or references to other document types that may be present in the subsequent text when evaluating the document type. Include the entire JSON in your response and do not include any introduction or final summary. I emphasize that a collection of links or references to another document type should not result in is_present being set to true. I emphasize again that you should evaluate document type based on characteristics of document itself and not whether the content in it refers to certain document types. For example a weather forecast document will contain numerical values for temperature forecasts, but a link to a weather forecast web page should not result in its classification as a 'weather' document. Add another sibling key to is_present called "justification" where you provide your reasoning for classifying the document. Thank you.
{ 
  "long_form": {
    "is_present": false,
    "criteria": "Must contain very large blocks of text split into sections/chapters like novels or textbooks"
  },
  "chat": {
    "is_present": false,
    "criteria": "Must contain mmall/medium sized user-generated text blocks representing content like: discussion forum posts, article comments, messenger chat"
  },
  "weather": {
    "is_present": false,
    "criteria": "weekly, daily forecasts for city or region"
  },
  "business_details": {
    "is_present": false,
    "criteria": "Information about a business like opening hours, address"
  },
  "curated_listing": {
    "is_present": false,
    "criteria": "User generated listing of urls from various sources. Perhaps containing voting/ranking, references to discussion/comments. Perhaps containing tags/categories."
  },
  "event_listing": {
    "is_present": false,
    "criteria": "Listing of dance events, concerts, etc."
  },
  "job_listing": {
    "is_present": false,
    "criteria": "Listing of jobs"
  },
  "real_estate_listing": {
    "is_present": false,
    "criteria": "Listing of properties for sale or rent"
  },
  "search_engine_listing": {
    "is_present": false,
    "criteria": "Listing of urls"
  }
}

The document to analyze:
"##;
