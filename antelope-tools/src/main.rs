use clap::{Parser, Subcommand};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

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
    WalletPassword,
}

fn init_tracing(verbose_level: u8) {
    // use an env filter with default level of INFO
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let tracing = tracing_subscriber::fmt()
        .with_env_filter(env_filter);

    // flags given on the command-line override those from the environment
    match verbose_level {
        0 => tracing.init(),
        1 => tracing.with_max_level(Level::DEBUG).init(),
        2 => tracing.with_max_level(Level::TRACE).init(),
        _ => panic!("too many -v flags, 2 max allowed"),
    };
}

fn main() {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

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
        Some(Commands::WalletPassword) => {
            let docker = Docker::new("eos_container".to_string(), "eos:latest".to_string());
            info!("Wallet password is: {}", &docker.get_wallet_password());
        }
        None => {},
    }
}
