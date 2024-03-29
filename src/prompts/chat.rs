pub static CHAT_GROUP_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent text and do your best to identify a pattern signifying posts/comments that people leave on websites such as discussion forums. These items are typically user generated, contain information like author, a body of text content, timestamp (relative or absolute), might contain points, parent and child comments as part of a larger thread and potentially much more. If you do see a set of distinct discussion posts, provide a regular expression that would capture each user generated post in the text. Do not provide an optimized regular expression, include as much redundant text that precedes or follows each post. Ensure that the regular expression matches across newline characters. Since dot-all mode or inline modifiers like (?s:.) are not supported, use a character class such as [.\n] or [\s\S] for this purpose. Additionally, avoid using lookahead and lookbehind constructs, as they may not be compatible with the regex engine in use. Print your response based on the following json:
{
    "pattern": "regex pattern goes here"
}
If the text does not contain any user discussion, print only the text 'false' and nothing else. Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CHAT_ITEM_PROMPT: &str = r##"
Hi ChatGPT. Your job is to interpret textual documents and to glean from it patterns that represent the salient information contained within these documents. Please examine the subsequent set of texts that we can expect to represent comments left on dicussion forums. These items often come from instant message tools, collaboration software, team chat, messaging apps, discussion forums, etc, where users can start new threads on a particular topic and engage in conversations with each other by posting messages. They may contain information representing whether they have a parent comment identifier. Very often there will be user/author information, a timestamp (relative or absolute), and potentially other fields. A comment will always have a body of text, so make sure to at least provide a regular expression for this key piece of information. Please identify common fields across all chat items and create regular expressions that would capture each field. I will provide multiple examples of the text to assist you in identifying when fields is static or dynamic. Each regular expression should contain at most one capturing group. Ensure regular expressions are not optimized and are as long as possible to ensure there is at most one match. If multiple elements need to be captured, provide separate regular expressions for each, ensuring only one capturing group is present in each expression. Ensure all urls (absolute or relative) have a corresponding regular expression, except for those in the text body of user-generated content. Set the key name to be the name of the field and its value should be the regular expression. Ensure that the regular expression matches across newline characters. Since dot-all mode or inline modifiers like (?s:.) are not supported, use a character class such as [.\n] or [\s\S] for this purpose. Additionally, avoid using lookahead and lookbehind constructs, as they may not be compatible with the regex engine in use. Print your response based on the following json:
{
    "fieldName": "regex pattern goes here",
    "otherField": "regex pattern goes here",
    ...
},
Please do not include any introduction or final summary in your response. Thank you.
"##;
pub static CHAT_ITEM_ADAPTER_PROMPT: &str = r##"
Hi ChatGPT. Your job is to map the keys of a JSON document I will provide you to the keys of a JSON template. The keys of the JSON I will provide should represent comments left on discussion forums, instant message tools, collaboration software, team chat, messaging apps. Please set the blank values of the JSON I will provide you to the keys of the following JSON object, based on them equivalently referring to the same thing but with a different name:
{
    "text": "The user-generated text body of comments",
    "author": "The user account that submitted the post/comment",
    "id": "The identifier of the item itself",
    "parent_id": "The parent comment identifier of a comment when messages exist in threads",
    "timestamp": "The timestamp of a comments, perhaps relative or absolute"
}
For example, if a key I provide is called "user", set its value to "author" as per the above guide. Use the values of this JSON object for more information about how to match the keys of the JSON document I will provide you. Ensure that the set of values you map keys to contains no duplicate values. For example if multiple keys seem to map to "parentId", only select one key to map to "parentId" and leave the others to map to their original values. If multiple keys seem to match above guide, use the most probably match and set the other keys to their original name. If the JSON document I provide contains keys that do not correspond to the above template, set the value to the original key name.
Please do not include any introduction or final summary in your response. Thank you.

JSON document:
"##;
