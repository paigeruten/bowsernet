use color_eyre::eyre::OptionExt;

#[derive(Debug, PartialEq)]
pub struct Url {
    pub scheme: Scheme,
    pub view_source: bool,
}

#[derive(Debug, PartialEq)]
pub enum Scheme {
    Http(HttpUrl),
    File(FileUrl),
    Data(DataUrl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpUrl {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
}

#[derive(Debug, PartialEq)]
pub struct FileUrl {
    pub path: String,
}

#[derive(Debug, PartialEq)]
pub struct DataUrl {
    pub content_type: String,
    pub contents: String,
}

impl Url {
    pub fn parse(url: &str) -> color_eyre::Result<Self> {
        let view_source = url.starts_with("view-source:");
        let url = url.strip_prefix("view-source:").unwrap_or(url);

        if let Some(url) = url.strip_prefix("data:") {
            let (content_type, contents) = url
                .split_once(',')
                .ok_or_eyre("Data URLs must have a content type")?;
            return Ok(Url {
                scheme: Scheme::Data(DataUrl {
                    content_type: content_type.to_string(),
                    contents: contents.to_string(),
                }),
                view_source,
            });
        }

        let (scheme, url) = url.split_once("://").ok_or_eyre("URL must have a scheme")?;

        if scheme == "file" {
            return Ok(Url {
                scheme: Scheme::File(FileUrl {
                    path: url.to_string(),
                }),
                view_source,
            });
        }

        let (mut host, url) = url.split_once('/').unwrap_or((url, ""));
        let path = format!("/{url}");

        let port = if let Some((actual_host, port)) = host.split_once(':') {
            host = actual_host;
            port.parse()?
        } else {
            match scheme {
                "http" => 80,
                "https" => 443,
                _ => return Err(color_eyre::eyre::eyre!("Scheme must be 'http' or 'https'.")),
            }
        };

        Ok(Url {
            scheme: Scheme::Http(HttpUrl {
                tls: scheme == "https",
                host: host.to_string(),
                port,
                path,
            }),
            view_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parse_http() {
        let expected = Url {
            scheme: Scheme::Http(HttpUrl {
                tls: false,
                host: "example.org".to_string(),
                port: 80,
                path: "/index.html".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(
            expected,
            Url::parse("http://example.org/index.html").unwrap()
        );
    }

    #[test]
    fn url_parse_http_with_no_path() {
        let expected = Url {
            scheme: Scheme::Http(HttpUrl {
                tls: false,
                host: "example.org".to_string(),
                port: 80,
                path: "/".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(expected, Url::parse("http://example.org").unwrap());
    }

    #[test]
    fn url_parse_http_with_explicit_port() {
        let expected = Url {
            scheme: Scheme::Http(HttpUrl {
                tls: false,
                host: "example.org".to_string(),
                port: 3000,
                path: "/index.html".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(
            expected,
            Url::parse("http://example.org:3000/index.html").unwrap()
        );
    }

    #[test]
    fn url_parse_http_with_view_source() {
        let expected = Url {
            scheme: Scheme::Http(HttpUrl {
                tls: false,
                host: "example.org".to_string(),
                port: 80,
                path: "/index.html".to_string(),
            }),
            view_source: true,
        };
        assert_eq!(
            expected,
            Url::parse("view-source:http://example.org/index.html").unwrap()
        );
    }

    #[test]
    fn url_parse_https() {
        let expected = Url {
            scheme: Scheme::Http(HttpUrl {
                tls: true,
                host: "example.org".to_string(),
                port: 443,
                path: "/index.html".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(
            expected,
            Url::parse("https://example.org/index.html").unwrap()
        );
    }

    #[test]
    fn url_parse_file() {
        let expected = Url {
            scheme: Scheme::File(FileUrl {
                path: "/etc/test.html".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(expected, Url::parse("file:///etc/test.html").unwrap());
    }

    #[test]
    fn url_parse_data() {
        let expected = Url {
            scheme: Scheme::Data(DataUrl {
                content_type: "text/html".to_string(),
                contents: "Hello world!".to_string(),
            }),
            view_source: false,
        };
        assert_eq!(expected, Url::parse("data:text/html,Hello world!").unwrap());
    }
}
