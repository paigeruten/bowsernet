use std::collections::HashMap;

#[derive(Debug)]
pub struct Headers {
    values: HashMap<String, HeaderValue>,
}

#[derive(Debug)]
struct HeaderValue {
    original_name: String,
    value: String,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn add(mut self, name: &str, value: &str) -> Self {
        self.set(name, value);
        self
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(
            name.to_ascii_lowercase(),
            HeaderValue {
                original_name: name.to_string(),
                value: value.to_string(),
            },
        );
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(&name.to_ascii_lowercase())
    }

    pub fn to_http_string(&self) -> String {
        let mut s = String::new();
        for header in self.values.values() {
            s.push_str(&format!("{}: {}\r\n", header.original_name, header.value));
        }
        s
    }
}
