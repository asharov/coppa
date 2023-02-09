use clap::Parser;
use coppa::Config;
use coppa::Distribution;
use coppa::Strategy;
use coppa::{EmptyRunObserver,SummaryRunObserver,DebugRunObserver};
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
    /// Seed to use for random number generation
    #[arg(long)]
    random_seed: Option<u64>,
    /// Do not print any progress reports
    #[arg(short='S', long)]
    silent: bool,
    /// Print verbose progress reports
    #[arg(short='V', long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_counts(cli.chunks, cli.peers, cli.seeds, cli.selfish, cli.freerider, cli.strategy);
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
    println!("");
    println!("Number of rounds {:?}", rounds.len() - 1);
    println!("Number of chunks exchanged {:?}", exchanged_chunks);
    println!("Execution time {:?}", execution_time);
}
