use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Create a new World
    Create { subject: String },
    /// Create a new BlockType in the current World
    BlockType,
    /// Create a new EntityType in the current World
    EntityType,
}

#[derive(Parser)]
struct Args {
    /// The broad command to run
    #[command(subcommand)]
    command: CliCommand,
}

fn main() {
    let args = Args::parse();

    match args.command {
        CliCommand::Create { ref subject } => do_create(subject, &args),
        CliCommand::BlockType => todo!(),
        CliCommand::EntityType => todo!(),
    }
}

fn do_create(_subject: &String, _args: &Args) {}
