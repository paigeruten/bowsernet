use bowsernet::{request, Url};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();

    let url = Url::parse(
        args.get(1)
            .unwrap_or(&"https://example.org/index.html".to_string()),
    )?;

    let response = request(&url)?;

    println!("{:?}", url);
    println!("{response}");

    Ok(())
}
