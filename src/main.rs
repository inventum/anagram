use clap::Parser;

fn main() -> anyhow::Result<()> {
    let code = anagram::run(anagram::cli::Cli::parse())?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}