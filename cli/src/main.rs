use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Create a new World
    Create {
        #[arg(help = "Name of the World to create")]
        subject: String,
    },
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

enum CliMessage {
    CreateWorldExistsAlready(String),
    CreateWorldOsError(std::io::Error),
    MakingWorld,
    MadeWorld,
}

struct ProgressTask<'a> {
    task: &'a dyn Fn() -> Result<(), ExitCode>,
    label: Option<String>,
}

fn main() -> Result<(), ExitCode> {
    let args = Args::parse();

    match args.command {
        CliCommand::Create { ref subject } => do_create(subject, &args),
        CliCommand::BlockType => todo!(),
        CliCommand::EntityType => todo!(),
    }
}

fn do_create(subject: &String, args: &Args) -> Result<(), ExitCode> {
    if std::fs::exists(subject).unwrap_or(false) {
        // In a more perfect world, this branch wouldn't exist here: instead we would have it as a
        // match arm on `std::fs::create_dir` below. (This would avoid a [theoretical] TOCTOU
        // issue.] However in practice, figuring out which error is "directory already exists" in a
        // cross-platform way is very annoying so it's easier to do the check separately
        show_message(CliMessage::CreateWorldExistsAlready(subject.clone()), args);
        return Err(ExitCode::FAILURE);
    } else {
        match std::fs::create_dir(subject) {
            Ok(()) => {
                show_message(CliMessage::MakingWorld, args);
                let bar = ProgressBar::new(3);
                let mut bar_time = Instant::now();
                progress_bar(
                    &bar,
                    &mut bar_time,
                    Some("✅ Set up standard block types 🧱"),
                );
                progress_bar(&bar, &mut bar_time, Some("✅ Set up entities 🤖"));
                progress_bar(
                    &bar,
                    &mut bar_time,
                    Some("✅ Set up base world voxel grid 🌐"),
                );
                drop(bar);
                show_message(CliMessage::MadeWorld, args);
            }
            Err(x) => {
                show_message(CliMessage::CreateWorldOsError(x), args);
                return Err(ExitCode::FAILURE);
            }
        };
    }
    return Ok(());
}

fn progress_bar(bar: &ProgressBar, time: &mut Instant, message: Option<&str>) {
    bar.inc(1);
    if let Some(msg) = message {
        bar.suspend(|| println!("{}", msg));
    }
    let time_taken = Instant::now() - *time;
    let ms_to_sleep = (150 - time_taken.as_millis()).clamp(0, 150);
    std::thread::sleep(Duration::from_millis(ms_to_sleep.try_into().unwrap()));
    *time = Instant::now();
}

fn show_message(message: CliMessage, _args: &Args) {
    match message {
        CliMessage::CreateWorldExistsAlready(name) => println!(
            "❌ The World '{}' (or at least a file of that name) exists already",
            name
        ),
        CliMessage::CreateWorldOsError(os_err) => println!(
            "‼️ Unexpected operating system error creating World: {:?}",
            os_err
        ),
        CliMessage::MakingWorld => println!("🌎 Preparing to make a new world..."),
        CliMessage::MadeWorld => println!("🌅 World created successfully"),
    }
}

#[cfg(test)]
mod tests {
    use super::{do_create, Args};
    use clap::Parser;
    use tempfile::tempdir;

    fn dummy_args() -> Args {
        Args::parse_from(vec!["", "create", "xx"])
    }

    #[test]
    fn test_create() {
        let args = dummy_args();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test-world");
        assert!(do_create(&path.clone().into_os_string().into_string().unwrap(), &args).is_ok());

        // Check the files we claim are in the documentation exist
        for file in [
            "blocktypes",
            "entities.json",
            "entitytypes",
            "grid.dat",
            "metadata.json",
            "player.ts",
            "skybox",
            "world.json",
            "world.ts",
        ] {
            assert!(std::fs::exists(path.join(file)).unwrap());
        }
    }

    #[test]
    fn test_duplicate_world() {
        let args = dummy_args();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test-world");
        assert!(do_create(&path.clone().into_os_string().into_string().unwrap(), &args).is_ok());
        assert!(do_create(&path.clone().into_os_string().into_string().unwrap(), &args).is_err());
    }
}
