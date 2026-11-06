use crate::cli::output::value_for_key;
use crate::config::Config;
use crate::info::Info;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Value {
    Text(String),
    List(Vec<String>),
}

impl Value {
    fn as_text(&self) -> String {
        match self {
            Value::Text(text) => text.clone(),
            Value::List(items) => items.join(", "),
        }
    }

    fn is_truthy(&self) -> bool {
        match self {
            Value::Text(text) => {
                let lower = text.trim().to_ascii_lowercase();
                !lower.is_empty() && !matches!(lower.as_str(), "false" | "0" | "unknown" | "n/a")
            }
            Value::List(items) => !items.is_empty(),
        }
    }

    fn as_list(&self) -> Vec<String> {
        match self {
            Value::Text(text) if text.is_empty() => Vec::new(),
            Value::Text(text) => vec![text.clone()],
            Value::List(items) => items.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    values: BTreeMap<String, Value>,
    this: Option<String>,
    index: Option<usize>,
}

impl Context {
    pub fn new(values: BTreeMap<String, Value>) -> Self {
        Self {
            values,
            this: None,
            index: None,
        }
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    fn truthy(&self, key: &str) -> bool {
        self.get(key).is_some_and(Value::is_truthy)
    }

    fn list(&self, key: &str) -> Vec<String> {
        self.get(key).map(Value::as_list).unwrap_or_default()
    }

    fn child(&self, item: String, index: usize) -> Self {
        let mut child = self.clone();
        child.this = Some(item);
        child.index = Some(index);
        child
    }
}

pub fn context(info: &Info, config: &Config, keys: &[String]) -> Context {
    let mut values = BTreeMap::new();
    for key in keys {
        let value = match key.as_str() {
            "gpu" => Value::List(info.gpu.clone()),
            "disks" => Value::List(info.disks.clone()),
            "packages_breakdown" => Value::List(
                info.packages_breakdown
                    .iter()
                    .map(|(manager, count)| format!("{manager}={count}"))
                    .collect(),
            ),
            key if key.starts_with("plugin:") => {
                Value::List(info.plugin_info.get(key).cloned().unwrap_or_default())
            }
            _ => Value::Text(value_for_key(info, config, key).unwrap_or_default()),
        };
        values.insert(key.clone(), value);
    }
    Context::new(values)
}

pub fn keys(template: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for token in tokenize(template) {
        let key = match token {
            Token::Var(tag) => tag.split('|').next().unwrap_or("").trim().to_string(),
            Token::If(key) | Token::Unless(key) | Token::Each(key) => key,
            _ => continue,
        };
        if !key.is_empty() && key != "this" && key != "@index" && !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

pub fn render(template: &str, context: &Context) -> String {
    render_tokens(&tokenize(template), context)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Text(String),
    Var(String),
    If(String),
    Unless(String),
    Each(String),
    Else,
    EndIf,
    EndUnless,
    EndEach,
}

fn tokenize(template: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let text = &rest[..start];
        if !text.is_empty() {
            tokens.push(Token::Text(text.to_string()));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            tokens.push(Token::Text(rest[start..].to_string()));
            return tokens;
        };
        let tag = after[..end].trim();
        let token = if let Some(key) = tag.strip_prefix("#if ") {
            Some(Token::If(key.trim().to_string()))
        } else if let Some(key) = tag.strip_prefix("#unless ") {
            Some(Token::Unless(key.trim().to_string()))
        } else if let Some(key) = tag.strip_prefix("#each ") {
            Some(Token::Each(key.trim().to_string()))
        } else if tag == "else" {
            Some(Token::Else)
        } else if tag == "/if" {
            Some(Token::EndIf)
        } else if tag == "/unless" {
            Some(Token::EndUnless)
        } else if tag == "/each" {
            Some(Token::EndEach)
        } else if tag.starts_with('!') {
            None
        } else {
            Some(Token::Var(tag.to_string()))
        };
        if let Some(token) = token {
            tokens.push(token);
        }
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        tokens.push(Token::Text(rest.to_string()));
    }
    tokens
}

fn render_tokens(tokens: &[Token], context: &Context) -> String {
    let mut output = String::new();
    let mut index = 0usize;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Text(text) => output.push_str(text),
            Token::Var(tag) => output.push_str(&resolve(tag, context)),
            Token::If(key) => {
                let (else_index, end_index) = scan_block(tokens, index + 1, "if");
                let true_end = else_index.unwrap_or(end_index);
                if context.truthy(key) {
                    output.push_str(&render_tokens(&tokens[index + 1..true_end], context));
                } else if let Some(else_index) = else_index {
                    output.push_str(&render_tokens(&tokens[else_index + 1..end_index], context));
                }
                index = end_index;
            }
            Token::Unless(key) => {
                let (else_index, end_index) = scan_block(tokens, index + 1, "unless");
                let true_end = else_index.unwrap_or(end_index);
                if !context.truthy(key) {
                    output.push_str(&render_tokens(&tokens[index + 1..true_end], context));
                } else if let Some(else_index) = else_index {
                    output.push_str(&render_tokens(&tokens[else_index + 1..end_index], context));
                }
                index = end_index;
            }
            Token::Each(key) => {
                let (else_index, end_index) = scan_block(tokens, index + 1, "each");
                let body_end = else_index.unwrap_or(end_index);
                let items = context.list(key);
                if items.is_empty() {
                    if let Some(else_index) = else_index {
                        output
                            .push_str(&render_tokens(&tokens[else_index + 1..end_index], context));
                    }
                } else {
                    for (item_index, item) in items.iter().enumerate() {
                        let child = context.child(item.clone(), item_index);
                        output.push_str(&render_tokens(&tokens[index + 1..body_end], &child));
                    }
                }
                index = end_index;
            }
            Token::Else | Token::EndIf | Token::EndUnless | Token::EndEach => {}
        }
        index += 1;
    }
    output
}

fn scan_block(tokens: &[Token], start: usize, kind: &'static str) -> (Option<usize>, usize) {
    let mut stack: Vec<&'static str> = vec![kind];
    let mut else_index = None;
    let mut index = start;
    while index < tokens.len() {
        match &tokens[index] {
            Token::If(_) => stack.push("if"),
            Token::Unless(_) => stack.push("unless"),
            Token::Each(_) => stack.push("each"),
            Token::EndIf => {
                if stack.last() == Some(&"if") {
                    stack.pop();
                    if stack.is_empty() {
                        return (else_index, index);
                    }
                }
            }
            Token::EndUnless => {
                if stack.last() == Some(&"unless") {
                    stack.pop();
                    if stack.is_empty() {
                        return (else_index, index);
                    }
                }
            }
            Token::EndEach => {
                if stack.last() == Some(&"each") {
                    stack.pop();
                    if stack.is_empty() {
                        return (else_index, index);
                    }
                }
            }
            Token::Else if stack.len() == 1 => else_index = Some(index),
            _ => {}
        }
        index += 1;
    }
    (else_index, tokens.len())
}

fn resolve(tag: &str, context: &Context) -> String {
    let mut parts = tag.split('|');
    let key = parts.next().unwrap_or("").trim();
    let mut value = if key == "this" {
        context
            .this
            .clone()
            .map(Value::Text)
            .unwrap_or(Value::Text(String::new()))
    } else if key == "@index" {
        Value::Text(
            context
                .index
                .map(|index| index.to_string())
                .unwrap_or_default(),
        )
    } else {
        context
            .get(key)
            .cloned()
            .unwrap_or(Value::Text(String::new()))
    };

    for filter in parts {
        let filter = filter.trim();
        let (name, argument) = match filter.split_once(':') {
            Some((name, argument)) => (name.trim(), Some(argument.trim().trim_matches('"'))),
            None => (filter, None),
        };
        value = match name {
            "upper" => Value::Text(value.as_text().to_uppercase()),
            "lower" => Value::Text(value.as_text().to_lowercase()),
            "trim" => Value::Text(value.as_text().trim().to_string()),
            "count" => Value::Text(value.as_list().len().to_string()),
            "join" => Value::Text(value.as_list().join(argument.unwrap_or(", "))),
            "default" => {
                if value.as_text().is_empty() {
                    Value::Text(argument.unwrap_or("").to_string())
                } else {
                    value
                }
            }
            _ => value,
        };
    }

    value.as_text()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        let mut values = BTreeMap::new();
        values.insert("os".to_string(), Value::Text("Arch Linux".to_string()));
        values.insert("empty".to_string(), Value::Text(String::new()));
        values.insert(
            "gpu".to_string(),
            Value::List(vec!["GPU A".to_string(), "GPU B".to_string()]),
        );
        Context::new(values)
    }

    #[test]
    fn variables_and_filters() {
        let context = context();
        assert_eq!(render("{{os}}", &context), "Arch Linux");
        assert_eq!(render("{{os|upper}}", &context), "ARCH LINUX");
        assert_eq!(render("{{os|lower}}", &context), "arch linux");
        assert_eq!(
            render("{{empty|default:\"fallback\"}}", &context),
            "fallback"
        );
        assert_eq!(render("{{gpu|count}}", &context), "2");
        assert_eq!(render("{{gpu|join:\" / \"}}", &context), "GPU A / GPU B");
    }

    #[test]
    fn conditionals() {
        let context = context();
        assert_eq!(render("{{#if os}}yes{{else}}no{{/if}}", &context), "yes");
        assert_eq!(render("{{#if empty}}yes{{else}}no{{/if}}", &context), "no");
        assert_eq!(render("{{#unless empty}}yes{{/unless}}", &context), "yes");
    }

    #[test]
    fn each_blocks() {
        let context = context();
        assert_eq!(
            render("{{#each gpu}}[{{@index}}] {{this}};{{/each}}", &context),
            "[0] GPU A;[1] GPU B;"
        );
        assert_eq!(
            render("{{#each empty}}x{{else}}none{{/each}}", &context),
            "none"
        );
    }

    #[test]
    fn keys_collects_all_references() {
        assert_eq!(
            keys("{{os}} {{#if cpu}}{{gpu}}{{/if}} {{#each disks}}{{this}}{{/each}}"),
            vec![
                "os".to_string(),
                "cpu".to_string(),
                "gpu".to_string(),
                "disks".to_string()
            ]
        );
    }
}
