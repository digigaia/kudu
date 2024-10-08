use clap::{Parser, Subcommand};
use tracing::{Level, info, trace};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

use dune::{Docker, Dune};

const CONTAINER_NAME: &str = "eos_container";
const IMAGE_NAME: &str = "eos:latest";


#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help(true))]
#[command(about = "Dune: Docker Utilities for Node Execution")]
struct Cli {
    /// Turn verbose level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Do not print any logging messages. Normal output of the command is
    /// still available on stdout. Use this when you want to make sure that
    /// the expected output will not be garbled by the logging messages
    /// (eg: if you're expecting some JSON output from the command)
    #[arg(short, long)]
    silent: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}


#[derive(Subcommand, Debug)]
enum Commands {
    // -----------------------------------------------------------------------------
    //     General commands
    // -----------------------------------------------------------------------------

    /// List all the Docker containers
    ListContainers,

    /// Build an EOS image starting from the given base image (default: ubuntu:22.04)
    BuildImage {
        #[arg(default_value = "ubuntu:22.04")]
        base: String
    },

    // -----------------------------------------------------------------------------
    //     Commands operating on a docker container
    // -----------------------------------------------------------------------------

    /// Start running nodeos in the current container
    StartNode {
        /// whether to replay the blockchain from the beginning when starting
        #[arg(short, long, default_value_t=false)]
        replay_blockchain: bool,

        /// whether to clean the datadir and start with a fresh one
        #[arg(short, long, default_value_t=false)]
        clean: bool,
    },

    /// Stop nodeos in the current container
    StopNode,

    /// Destroy the current EOS container
    Destroy {
        #[arg(default_value=CONTAINER_NAME)]
        container_name: Option<String>,
    },

    /// Bootstrap a running system by installing the system contracts etc. FIXME desc
    Bootstrap {
        /// full also deploys [...] FIXME desc!
        #[arg(short, long, default_value_t=false)]
        full: bool
    },

    /// Create a new account on the blockchain with initial resources
    #[command(name="system-newaccount")]
    SystemNewAccount {
        /// The name for the new account
        account: String,
        /// The name of the creator of the account
        #[arg(default_value="eosio")]
        creator: Option<String>,
    },

    /// Deploy a compiled contract to the blockchain
    DeployContract {
        /// The folder where the contract is located
        location: String,
        /// The account name on which to deploy the contract
        account: String,
    },

    /// Build the cmake project in the given directory
    CmakeBuild {
        /// The source directory containing the project
        location: String,
    },

    /// Show the wallet password
    WalletPassword,

    /// Pass-through that runs the given command in the container
    Exec {
        /// The commands you want to execute and its arguments
        cmd: Vec<String>,
    },
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

    if !cli.silent {
        init_tracing(cli.verbose);
        trace!("{:?}", cli);  // FIXME: temporary
    }

    // let container_name = "eos_container";
    // let image_name = "eos:latest";

    let Some(cmd) = cli.command else { unreachable!("no command -> show help"); };

    // first check the commands that don't need an instance of a Dune docker runner
    // this avoids building and starting a container when it is not needed
    match cmd {
        Commands::ListContainers => {
            for c in Docker::list_all_containers().iter() {
                let name = c["Names"].to_string();
                let status = c["Status"].as_str().unwrap();
                println!("Container: {:20} ({})", name, status);
            }
        },
        Commands::BuildImage { base } => {
            Dune::build_image(IMAGE_NAME, &base);
        },
        Commands::Destroy { container_name } => {
            Docker::destroy(container_name.as_deref()
                            .expect("has default value"));
        },
        // all the other commands need a `Dune` instance, get one now and keep matching
        _ => {
            let dune = Dune::new(CONTAINER_NAME.to_string(), IMAGE_NAME.to_string());

            match cmd {
                Commands::WalletPassword => {
                    info!("Wallet password is:");
                    println!("{}", &dune.get_wallet_password());
                },
                Commands::StartNode { replay_blockchain, clean } => {
                    dune.start_node(replay_blockchain, clean);
                },
                Commands::StopNode => {
                    dune.stop_node();
                },
                Commands::Bootstrap { full } => {
                    dune.bootstrap_system(full);
                },
                Commands::SystemNewAccount { account, creator } => {
                    dune.system_newaccount(&account, creator.as_deref()
                                           .expect("has default value"));
                },
                Commands::DeployContract { location, account } => {
                    dune.deploy_contract(&Docker::abs_host_path(&location), &account);
                },
                Commands::CmakeBuild { location } => {
                    dune.cmake_build(&location);
                },
                Commands::Exec { cmd } => {
                    // FIXME: we should deactivate all forms of logging before getting here
                    // otherwise we can get our stdout (which holds the result of the command)
                    // polluted by our own logging
                    let cmd: Vec<_> = cmd.iter().map(String::as_str).collect();
                    dune.command(&cmd).capture_output(false).run();
                }
                _ => todo!(),
            }
        }
    }
}
