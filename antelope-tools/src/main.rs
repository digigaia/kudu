use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use antelope_tools::docker::Docker;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn verbose level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    ListContainers,
    BuildImage,
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

fn main() {
    init_tracing();

    let cli = Cli::parse();

    // TODO: use that to set the log levels, overriding the env if needed
    match cli.verbose {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Test { list }) => {
            if *list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        },
        Some(Commands::ListContainers) => {
            let docker = Docker::new("eos_container".to_string(), "eos:latest".to_string());
            for c in docker.list_all_containers().iter() {
                let name = c["Names"].to_string();
                let status = c["Status"].as_str().unwrap();
                println!("Container: {:20} ({})", name, status);
            }
        },
        Some(Commands::BuildImage) => {
            let docker = Docker::new("eos_container".to_string(), "eos:latest".to_string());
            docker.build_image();
        }
        None => {},
    }
}
