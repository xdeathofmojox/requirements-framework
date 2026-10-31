use clap::Parser;
use requirements_viewer::{Store, Validation, router};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Local Axum requirements viewer; --edit enables validated file additions")]
struct Arguments {
    #[arg(long, conflicts_with = "schema_version")]
    schema: Option<PathBuf>,
    #[arg(long)]
    schema_version: Option<String>,
    #[arg(
        long,
        env = "REQUIREMENTS_VALIDATOR",
        default_value = "requirements-validate",
        hide = true
    )]
    validator: PathBuf,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long)]
    no_browser: bool,
    #[arg(long)]
    edit: bool,
    /// Requirement JSON files or folders to scan recursively.
    #[arg(required = true, value_name = "FILE_OR_FOLDER")]
    documents: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();
    let validation = Validation {
        executable: args.validator,
        schema: args.schema,
        schema_version: args.schema_version,
    };
    let store = Store::open(args.documents, validation, args.edit)?;
    let listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args.port)).await?;
    let authority = listener.local_addr()?.to_string();
    let url = format!("http://{authority}/");
    println!(
        "Requirements viewer: {url}\n{}\nPress Ctrl+C to stop.",
        if args.edit {
            "Additions enabled for opened files."
        } else {
            "Read only; use --edit to add requirements."
        }
    );
    if !args.no_browser
        && let Err(error) = webbrowser::open(&url)
    {
        eprintln!("Open the URL manually: {error}");
    }
    axum::serve(listener, router(store, authority))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
