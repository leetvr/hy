use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum, Debug)]
enum CliCommand {
    /// Create a new World
    Create,
    /// Create a new BlockType in the current World
    BlockType,
    /// Create a new EntityType in the current World
    EntityType,
}

#[derive(Parser)]
struct Args {
    /// The broad command to run
    command: CliCommand,
    /// The main subject of the command, if any
    subject: Option<String>,
}

fn main() {
    let args = Args::parse();

    println!("command -: {:?}", args.command);
}
