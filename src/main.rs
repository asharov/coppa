use coppa::Distribution;
use coppa::SummaryRunObserver;

fn main() {
    let mut distribution = Distribution::new(10, 2, 10);
    let rounds = distribution.run(None, SummaryRunObserver);
    let mut exchanged_chunks = 0;
    for round in &rounds {
        exchanged_chunks += round.exchanged_chunks
    }
    println!("");
    println!("Number of rounds {:?}", rounds.len() - 1);
    println!("Number of chunks exchanged {:?}", exchanged_chunks);
}
