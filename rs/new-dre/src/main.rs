use clap::Parser;
use commands::Args;

mod commands;

#[tokio::main]
async fn main() {
    let _args = Args::parse();

    println!("Done.")
}
