use color_eyre::eyre::OptionExt;

#[derive(Debug)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub path: String,
}

impl Url {
    pub fn parse(url: &str) -> color_eyre::Result<Self> {
        let (scheme, url) = url.split_once("://").ok_or_eyre("URL must have a scheme")?;
        let (host, url) = url.split_once('/').unwrap_or((url, ""));
        let path = format!("/{url}");

        Ok(Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            path,
        })
    }
}
