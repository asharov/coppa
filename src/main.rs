use clap::Parser;
use coppa::Distribution;
use coppa::Strategy;
use coppa::{Config, PeerConfig};
use coppa::{DebugRunObserver, EmptyRunObserver, SummaryRunObserver};
use std::fs;
use std::time::Duration;

#[derive(Parser, Debug)]
struct Cli {
    /// Number of chunks in the distributed file
    #[arg(short, long)]
    chunks: usize,
    /// Total number of participating peers (including seeds)
    #[arg(short, long)]
    peers: usize,
    /// Number of seeds
    #[arg(short, long, default_value_t = 1)]
    seeds: usize,
    /// Number of peers that stop distributing after completion
    #[arg(long, default_value_t = 0)]
    selfish: usize,
    /// Number of peers that do not distribute
    #[arg(long, default_value_t = 0)]
    freerider: usize,
    /// Chunk selection strategy that all peers use
    #[arg(long, value_enum, default_value_t = Strategy::RarestFirst)]
    strategy: Strategy,
    /// The fast network speed
    #[arg(long)]
    speed_fast: Option<usize>,
    /// The medium network speed
    #[arg(long)]
    speed_medium: Option<usize>,
    /// The slow network speed
    #[arg(long)]
    speed_slow: Option<usize>,
    /// File containing peer configuration, one peer per line
    #[arg(short = 'F', long)]
    peer_config_file: Option<String>,
    /// Seed to use for random number generation
    #[arg(long)]
    random_seed: Option<u64>,
    /// Do not print any progress reports
    #[arg(short = 'S', long)]
    silent: bool,
    /// Print verbose progress reports
    #[arg(short = 'V', long)]
    verbose: bool,
}

impl Cli {
    pub fn assert_consistency(&self) {
        if self.peer_config_file.is_some() {
            assert!(self.selfish == 0);
            assert!(self.freerider == 0);
            assert!(self.strategy == Strategy::RarestFirst);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    cli.assert_consistency();
    let speed_slow = cli.speed_slow.unwrap_or(1);
    let speed_medium = cli.speed_medium.unwrap_or(speed_slow);
    let speed_fast = cli.speed_fast.unwrap_or(speed_medium);
    let config = if let Some(peer_config_file) = cli.peer_config_file {
        let mut peer_config_contents = fs::read(peer_config_file.clone())
            .unwrap_or_else(|_| panic!("Could not read file {peer_config_file}"));
        peer_config_contents.truncate(peer_config_contents.len() - 1);
        let peer_config_strings = peer_config_contents.split(|c| *c == b'\n');
        let peer_config = peer_config_strings.map(PeerConfig::from_string).collect();
        Config::from_peer_config(
            cli.chunks,
            cli.peers,
            cli.seeds,
            speed_fast,
            speed_medium,
            speed_slow,
            peer_config,
        )
    } else {
        Config::from_counts(
            cli.chunks,
            cli.peers,
            cli.seeds,
            speed_fast,
            speed_medium,
            speed_slow,
            cli.selfish,
            cli.freerider,
            cli.strategy,
        )
    };
    let mut distribution = Distribution::new(&config);
    let rounds = if cli.silent {
        distribution.run(cli.random_seed, EmptyRunObserver)
    } else if cli.verbose {
        distribution.run(cli.random_seed, DebugRunObserver)
    } else {
        distribution.run(cli.random_seed, SummaryRunObserver)
    };
    let mut exchanged_chunks = 0;
    let mut execution_time = Duration::from_secs(0);
    for round in &rounds {
        exchanged_chunks += round.exchanged_chunks;
        execution_time += round.execution_time;
    }
    println!();
    println!("Number of rounds {:?}", rounds.len() - 1);
    println!("Number of chunks exchanged {exchanged_chunks:?}");
    println!("Execution time {execution_time:?}");
}
