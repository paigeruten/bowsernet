use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use bowsernet::{request, show, ConnectionPool, RequestCache, Url};

const DEFAULT_URL: &str = "file://examples/welcome.html";

fn load(url: &Url) -> color_eyre::Result<()> {
    let mut connection_pool = ConnectionPool::new();
    let mut request_cache = RequestCache::new();
    let body = request(url, &mut connection_pool, &mut request_cache)?;
    show(&body);
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    setup_tracing()?;

    let args: Vec<String> = std::env::args().collect();

    let url = Url::parse(args.get(1).unwrap_or(&DEFAULT_URL.to_string()))?;

    load(&url)?;

    Ok(())
}

pub fn setup_tracing() -> color_eyre::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let format = tracing_subscriber::fmt::format().pretty();
    let formatting_layer = tracing_subscriber::fmt::layer().event_format(format);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .init();
    Ok(())
}
