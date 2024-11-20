use bowsernet::{request, show, Url};

fn load(url: &Url) -> color_eyre::Result<()> {
    let body = request(url)?;
    show(&body);
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();

    let url = Url::parse(
        args.get(1)
            .unwrap_or(&"https://example.org/index.html".to_string()),
    )?;

    load(&url)?;

    Ok(())
}
