use color_eyre::eyre::OptionExt;

#[derive(Debug)]
pub struct Url {
    pub scheme: Scheme,
}

#[derive(Debug)]
pub enum Scheme {
    Http(HttpUrl),
    File(FileUrl),
}

#[derive(Debug)]
pub struct HttpUrl {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
}

#[derive(Debug)]
pub struct FileUrl {
    pub path: String,
}

impl Url {
    pub fn parse(url: &str) -> color_eyre::Result<Self> {
        let (scheme, url) = url.split_once("://").ok_or_eyre("URL must have a scheme")?;

        if scheme == "file" {
            return Ok(Url {
                scheme: Scheme::File(FileUrl {
                    path: url.to_string(),
                }),
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
        })
    }
}
