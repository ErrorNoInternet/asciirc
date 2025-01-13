use clap::Parser;

/// Send ASCII art on IRC using multiple clients
#[derive(Parser)]
#[command(about, version)]
pub struct Arguments {
    /// IRC server to connect to
    #[arg(short, long)]
    pub server: String,

    /// IRC nickname to use
    #[arg(short, long)]
    pub nickname: String,

    /// IRC channel to join
    #[arg(short = 'C', long)]
    pub channel: String,

    /// Amount of IRC clients to use
    #[arg(short, long, default_value_t = 5)]
    pub clients: usize,

    /// Nicknames allowed to use the bot
    #[arg(short, long, required = true)]
    pub owners: Vec<String>,

    /// Milliseconds to wait for the previous line to be received
    #[arg(short, long, default_value_t = 1000)]
    pub line_timeout: u64,
}
