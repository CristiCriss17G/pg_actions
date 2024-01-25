use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets Postgres connection URL
    #[arg(short = 'H', long, env, global = true)]
    hostname: Option<String>,

    /// Sets Postgres username
    #[arg(short = 'U', long, env, global = true)]
    username: Option<String>,

    /// Sets Postgres password
    #[arg(short, long, env = "PG_PASS", global = true)]
    password: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    debug: u8,

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
    /// does testing2 things
    Test2 {
        /// lists test values
        #[arg(short, long)]
        list: bool,
        #[command(subcommand)]
        subcommand: Option<TestSubCommands>,
    },
}

#[derive(Subcommand)]
enum TestSubCommands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
    /// does testing2 things
    Test2 {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(hostname) = cli.hostname.as_deref() {
        println!("Value for hostname: {}", hostname);
    }

    if let Some(username) = cli.username.as_deref() {
        println!("Value for username: {}", username);
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match cli.debug {
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
        }
        Some(Commands::Test2 { list, subcommand }) => {
            if *list {
                println!("Printing testing2 lists...");
            } else {
                println!("Not printing testing2 lists...");
            }
            match subcommand {
                Some(TestSubCommands::Test { list }) => {
                    if *list {
                        println!("Printing testing lists...");
                    } else {
                        println!("Not printing testing lists...");
                    }
                }
                Some(TestSubCommands::Test2 { list }) => {
                    if *list {
                        println!("Printing testing2 lists...");
                    } else {
                        println!("Not printing testing2 lists...");
                    }
                }
                None => {}
            }
        }
        None => {}
    }

    // Continued program logic goes here...
}
