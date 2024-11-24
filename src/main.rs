use bowsernet::{request, show, ConnectionPool, Url};

const DEFAULT_URL: &str = "file://examples/welcome.html";

fn load(url: &Url) -> color_eyre::Result<()> {
    let mut connection_pool = ConnectionPool::new();
    let body = request(url, &mut connection_pool)?;
    show(&body);
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();

    let url = Url::parse(args.get(1).unwrap_or(&DEFAULT_URL.to_string()))?;

    load(&url)?;

    Ok(())
}
