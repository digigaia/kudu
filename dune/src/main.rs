use clap::{Parser, Subcommand};
use tracing::{Level, debug, info};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use dune::{Docker, Dune};


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn verbose level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },

    /// Show help
    //Help,

    /// List all the Docker containers
    ListContainers,

    /// Build an EOS image starting from the given base image (default: ubuntu:22.04)
    BuildImage {
        #[arg(default_value = "ubuntu:22.04")]
        base: String
    },

    /// Show the wallet password
    WalletPassword,

    /// Start running nodeos in the current container
    StartNode {
        /// whether to replay the blockchain from the beginning when starting
        #[arg(short, long, default_value_t=false)]
        replay_blockchain: bool,
    },

    /// Stop nodeos in the current container
    StopNode,
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

    debug!("{:?}", cli);

    // init_tracing(cli.verbose);
    init_tracing(2);  // FIXME: temp

    let container_name = "eos_container";
    let image_name = "eos:latest";

    let cmd = match cli.command {
        Some(command) => command,
        None => Commands::ListContainers, // TODO: we want to show the help here
    };

    // first check the commands that don't need an instance of a Dune docker runner
    // this avoids building and starting a container when it is not needed
    match cmd {
        Commands::Test { list } => {
            if list {
                println!("Printing testing lists...");
            } else {
                println!("Not printing testing lists...");
            }
        },
        Commands::ListContainers => {
            for c in Docker::list_all_containers().iter() {
                let name = c["Names"].to_string();
                let status = c["Status"].as_str().unwrap();
                println!("Container: {:20} ({})", name, status);
            }
        },
        Commands::BuildImage { base } => {
            Dune::build_image(image_name, &base);
        },
        // all the other commands need a `Dune` instance, get one now and keep matching
        _ => {
            let dune = Dune::new(container_name.to_string(), image_name.to_string());

            match cmd {
                Commands::WalletPassword => {
                    info!("Wallet password is: {}", &dune.get_wallet_password());
                },
                Commands::StartNode { replay_blockchain } => {
                    dune.start_node(replay_blockchain);
                },
                Commands::StopNode => {
                    dune.stop_node();
                },
                // Commands::Help => {
                //     todo!();
                // }
                _ => todo!(),
            }
        }
    }

}
