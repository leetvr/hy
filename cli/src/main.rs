use clap::{Parser, Subcommand};
use console::Style;
use indicatif::ProgressBar;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Create a new World
    Create {
        #[arg(help = "Name of the World to create")]
        subject: String,
    },
    /// Create a new BlockType in the current World
    BlockType {
        #[arg(help = "Name of the new BlockType to create")]
        subject: String,
    },
    /// Create a new EntityType in the current World
    EntityType,
    /// Start the Hytopia Development Server
    #[command(name = "run")]
    RunServer {
        #[arg(help = "Name of the World to load into the development server")]
        subject: String,
    },
    /// Load up the web browser
    #[command(name = "dev")]
    LoadWebBrowser,
}

#[derive(Parser)]
struct Args {
    /// The broad command to run
    #[command(subcommand)]
    command: CliCommand,
    /// Go as fast as possible rather than optimising output for readability
    #[arg(long)]
    fast_as_possible: bool,
}

enum CliMessage {
    BlockTypeExistsAlready(String, String),
    CreateWorldExistsAlready(String),
    CreateWorldOsError(std::io::Error),
    MakingWorld,
    MakingBlockType,
    MadeWorld(String),
}

/// Wrapper for some useful progress bar functionality as we use it in this app.
///
/// In particular, this internally manages the delay, and has some useful helper functions
struct EmojiProgressBar {
    bar: ProgressBar,
    last_updated: Instant,
    min_task_time_ms: u128,
}

impl EmojiProgressBar {
    pub fn new(items: u64, args: &Args) -> Self {
        EmojiProgressBar {
            bar: ProgressBar::new(items),
            last_updated: Instant::now(),
            min_task_time_ms: if args.fast_as_possible { 0 } else { 150 },
        }
    }

    pub fn do_progress(&mut self, message: Option<&str>) {
        self.bar.inc(1);
        if let Some(msg) = message {
            self.bar.suspend(|| println!("{}", msg));
        }
        let time_taken = Instant::now() - self.last_updated;
        let ms_to_sleep =
            (self.min_task_time_ms - time_taken.as_millis()).clamp(0, self.min_task_time_ms);
        // >1 rather than >0 to avoid nuisance sleeps
        if ms_to_sleep > 1 {
            std::thread::sleep(Duration::from_millis(ms_to_sleep.try_into().unwrap()));
        }
        self.last_updated = Instant::now();
    }
}

fn main() -> Result<(), ExitCode> {
    let args = Args::parse();

    match args.command {
        CliCommand::Create { ref subject } => do_create(subject, &args),
        CliCommand::BlockType { ref subject } => do_new_blocktype(subject, &args),
        CliCommand::EntityType => todo!(),
        CliCommand::RunServer { ref subject } => do_run_server(subject, &args),
        CliCommand::LoadWebBrowser => do_load_web_browser(&args),
    }
}

fn do_new_blocktype(blocktype_name: &String, args: &Args) -> Result<(), ExitCode> {
    // TODO: Figure out what world we're in
    // TODO -- add an arg to specify which arg
    let the_world = "kibble_ctf";
    let mut path = PathBuf::new();
    path.push(the_world);
    path.push(blocktype_name);
    if std::fs::exists(path).unwrap_or(false) {
        show_message(
            CliMessage::BlockTypeExistsAlready(String::from(the_world), blocktype_name.clone()),
            args,
        );
        return Err(ExitCode::FAILURE);
    } else {
        show_message(CliMessage::MakingBlockType, args);
    }
    Ok(())
}

#[allow(dead_code)]
fn do_new_entity_type() -> Result<(), ExitCode> {
    Ok(())
}

fn do_run_server(_subject: &String, _args: &Args) -> Result<(), ExitCode> {
    Command::new("cargo")
        // TODO: not just k_ctf
        .args(["run", "--bin", "server", "kibble_ctf"])
        // Suppress browser if we're just asked to run the server
        .env("BROWSER", "none")
        .status()
        .map_err(|_| ExitCode::FAILURE)?;
    Ok(())
}

fn do_load_web_browser(_args: &Args) -> Result<(), ExitCode> {
    webbrowser::open("http://localhost:8888").map_err(|_| ExitCode::FAILURE)?;
    Ok(())
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
                let mut bar = EmojiProgressBar::new(3, args);
                bar.do_progress(Some("✅ Set up standard block types 🧱"));
                bar.do_progress(Some("✅ Set up entities 🤖"));
                bar.do_progress(Some("✅ Set up base world voxel grid 🌐"));
                drop(bar);
                show_message(CliMessage::MadeWorld(subject.clone()), args);

                if !args.fast_as_possible {
                    show_countdown("Starting development server in");
                }
                println!("Starting development server...");

                Command::new("cargo")
                    .args(["run", "--bin", "server", "kibble_ctf"])
                    .status()
                    .map_err(|_| ExitCode::FAILURE)?;
            }
            Err(x) => {
                show_message(CliMessage::CreateWorldOsError(x), args);
                return Err(ExitCode::FAILURE);
            }
        };
    }
    return Ok(());
}

/// Show a countdown for three seconds
///
/// Bail out if (a) the output is not a terminal, or (b) we get any error responses from
/// console::Term
fn show_countdown(message: &str) {
    let term = console::Term::stdout();
    if !term.is_term() {
        return;
    }
    for i in 0..3 {
        if let Err(_) = term.write_line(&format!("{} {}...", message, 3 - i)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1000));
        if let Err(_) = term.move_cursor_up(1) {
            return;
        }
        if let Err(_) = term.clear_line() {
            return;
        }
    }
}

fn show_message(message: CliMessage, _args: &Args) {
    let horiz = Style::new()
        .color256(197)
        .apply_to("~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~=~");
    let code_style = Style::new().color256(201).underlined();
    let link_style = Style::new().color256(39).underlined();
    match message {
        CliMessage::BlockTypeExistsAlready(world, name) => println!(
            "❌ The Block Type '{}' exists in the world '{}' already",
            world, name
        ),
        CliMessage::CreateWorldExistsAlready(name) => println!(
            "❌ The World '{}' (or at least a file of that name) exists already",
            name
        ),
        CliMessage::CreateWorldOsError(os_err) => println!(
            "‼️ Unexpected operating system error creating World: {:?}",
            os_err
        ),
        CliMessage::MakingWorld => println!("🌎 Preparing to make a new world..."),
        CliMessage::MakingBlockType => {
            println!("🔨 Writing out block type template...\nComplete. New block type ready.")
        }
        CliMessage::MadeWorld(name) => println!(
            r#"{horiz}
🌅 World created successfully

Your new world is located in `{}/`

In a second, I’ll start the development server and load up the Hytopia Editor in your web browser.

Remember you can always:
 👉 Start the development server: run `{}`
 👉 Use the Hytopia Editor: run `{}' or visit {}
 👉 Check out the documentation: {}

{}! Actually I’ll just start kibble_ctf.
"#,
            code_style.apply_to(name.clone()),
            code_style.apply_to(format!("hy run {}", name)),
            code_style.apply_to(format!("hy dev {}", name)),
            link_style.apply_to("https://localhost:8888/where/"),
            link_style.apply_to("https://linktogohere.invalid/docs/"),
            console::style("HAHA TRICKED U").red(),
        ),
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
