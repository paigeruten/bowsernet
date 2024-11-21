use color_eyre::eyre::OptionExt;

#[derive(Debug)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Url {
    pub fn parse(url: &str) -> color_eyre::Result<Self> {
        let (scheme, url) = url.split_once("://").ok_or_eyre("URL must have a scheme")?;
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

        Ok(Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
            path,
        })
    }
}
