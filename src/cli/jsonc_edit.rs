use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Key(String),
    Index(usize),
}

pub fn parse_path(path: &str) -> Vec<Segment> {
    path.split('.')
        .filter(|part| !part.is_empty())
        .map(|part| match part.parse::<usize>() {
            Ok(index) => Segment::Index(index),
            Err(_) => Segment::Key(part.to_string()),
        })
        .collect()
}

pub fn replace_value_preserving(text: &str, path: &[Segment], new_value: &str) -> Option<String> {
    let mut parser = Parser::new(text);
    let span = parser.find(path)?;
    let mut output = String::with_capacity(text.len() + new_value.len());
    output.push_str(&text[..span.start]);
    output.push_str(new_value);
    output.push_str(&text[span.end..]);
    Some(output)
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn skip_ws(&mut self) {
        loop {
            while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
                self.pos += 1;
            }
            if self.bytes.get(self.pos..self.pos + 2) == Some(b"//") {
                self.pos += 2;
                while let Some(byte) = self.peek() {
                    self.pos += 1;
                    if byte == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if self.bytes.get(self.pos..self.pos + 2) == Some(b"/*") {
                self.pos += 2;
                while self.pos + 1 < self.bytes.len() {
                    if &self.bytes[self.pos..self.pos + 2] == b"*/" {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn parse_string(&mut self) -> Option<String> {
        if self.peek() != Some(b'"') {
            return None;
        }
        let start = self.pos;
        self.pos += 1;
        while let Some(byte) = self.bump() {
            match byte {
                b'\\' => {
                    self.bump();
                }
                b'"' => {
                    let raw = &self.text[start..self.pos];
                    return serde_json::from_str::<String>(raw).ok();
                }
                _ => {}
            }
        }
        None
    }

    fn skip_string(&mut self) -> bool {
        if self.peek() != Some(b'"') {
            return false;
        }
        self.pos += 1;
        while let Some(byte) = self.bump() {
            match byte {
                b'\\' => {
                    self.bump();
                }
                b'"' => return true,
                _ => {}
            }
        }
        false
    }

    fn skip_value(&mut self) -> bool {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.skip_object(),
            Some(b'[') => self.skip_array(),
            Some(b'"') => self.skip_string(),
            Some(_) => {
                while let Some(byte) = self.peek() {
                    if byte.is_ascii_whitespace() || matches!(byte, b',' | b'}' | b']') {
                        break;
                    }
                    if byte == b'/'
                        && matches!(self.bytes.get(self.pos + 1), Some(b'/') | Some(b'*'))
                    {
                        break;
                    }
                    self.pos += 1;
                }
                true
            }
            None => false,
        }
    }

    fn skip_object(&mut self) -> bool {
        if self.bump() != Some(b'{') {
            return false;
        }
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return true;
            }
            if !self.skip_string() {
                return false;
            }
            self.skip_ws();
            if self.bump() != Some(b':') {
                return false;
            }
            if !self.skip_value() {
                return false;
            }
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn skip_array(&mut self) -> bool {
        if self.bump() != Some(b'[') {
            return false;
        }
        loop {
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return true;
            }
            if !self.skip_value() {
                return false;
            }
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn find(&mut self, path: &[Segment]) -> Option<Range<usize>> {
        self.skip_ws();
        if path.is_empty() {
            let start = self.pos;
            if self.skip_value() {
                return Some(start..self.pos);
            }
            return None;
        }

        match &path[0] {
            Segment::Key(key) => {
                if self.bump() != Some(b'{') {
                    return None;
                }
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b'}') {
                        return None;
                    }
                    let found_key = self.parse_string()?;
                    self.skip_ws();
                    if self.bump() != Some(b':') {
                        return None;
                    }
                    if found_key == *key {
                        return self.find(&path[1..]);
                    }
                    if !self.skip_value() {
                        return None;
                    }
                    self.skip_ws();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => return None,
                        _ => return None,
                    }
                }
            }
            Segment::Index(index) => {
                if self.bump() != Some(b'[') {
                    return None;
                }
                let mut current = 0usize;
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b']') {
                        return None;
                    }
                    if current == *index {
                        return self.find(&path[1..]);
                    }
                    if !self.skip_value() {
                        return None;
                    }
                    self.skip_ws();
                    match self.peek() {
                        Some(b',') => {
                            self.pos += 1;
                            current += 1;
                        }
                        Some(b']') => return None,
                        _ => return None,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_a_nested_scalar_and_keeps_comments() {
        let text = "{\n  // keep\n  \"colors\": {\n    \"cpu\": \"red\", // note\n    \"os\": \"cyan\"\n  }\n}\n";
        let path = parse_path("colors.cpu");
        let output = replace_value_preserving(text, &path, "\"green\"").unwrap();
        assert!(output.contains("// keep"));
        assert!(output.contains("// note"));
        assert!(output.contains("\"cpu\": \"green\""));
    }

    #[test]
    fn replaces_an_array_element() {
        let text = "{ \"modules\": [\"os\", \"cpu\", \"memory\"] }";
        let path = parse_path("modules.1");
        let output = replace_value_preserving(text, &path, "\"kernel\"").unwrap();
        assert_eq!(output, "{ \"modules\": [\"os\", \"kernel\", \"memory\"] }");
    }

    #[test]
    fn rejects_missing_paths() {
        let text = "{ \"colors\": { \"cpu\": \"red\" } }";
        assert!(replace_value_preserving(text, &parse_path("colors.gpu"), "\"blue\"").is_none());
    }

    #[test]
    fn handles_escaped_keys() {
        let text = "{ \"a\\\"b\": 1 }";
        let output = replace_value_preserving(text, &parse_path("a\"b"), "2").unwrap();
        assert_eq!(output, "{ \"a\\\"b\": 2 }");
    }
}
