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

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(&name.to_ascii_lowercase())
            .map(|value| value.value.as_str())
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

#[derive(Debug, Default)]
pub struct CacheControl {
    pub no_store: bool,
    pub max_age: Option<u64>,
}

impl From<&str> for CacheControl {
    fn from(value: &str) -> Self {
        let mut no_store = false;
        let mut max_age = None;
        for directive in value.split(',') {
            let directive = directive.trim_ascii().to_lowercase();
            if directive == "no-store" {
                no_store = true;
            } else if let Some(max_age_value) = directive.strip_prefix("max-age=") {
                max_age = max_age_value.parse().ok().filter(|&max_age| max_age != 0);
            }
        }

        CacheControl { no_store, max_age }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_keyed_by_lowercase() {
        let headers = Headers::new().add("Host", "example.org");

        assert!(headers.contains("Host"));
        assert!(headers.contains("host"));
        assert!(!headers.contains("nonexistent"));
    }

    #[test]
    fn headers_set_overwrites_or_adds() {
        let mut headers = Headers::new();
        headers.set("Host", "example.org");
        headers.set("host", "example.com");

        assert_eq!(Some("example.com"), headers.get("Host"));
        assert_eq!(Some("example.com"), headers.get("host"));
    }

    #[test]
    fn headers_to_http_string() {
        let headers = Headers::new()
            .add("Host", "example.org")
            .add("Connection", "close");

        let http_string = headers.to_http_string();

        assert!(
            http_string == "Host: example.org\r\nConnection: close\r\n"
                || http_string == "Connection: close\r\nHost: example.org\r\n"
        );
    }
}
