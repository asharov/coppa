use chrono::Utc;
use clap::ValueEnum;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Selfishness {
    Altruistic,
    Selfish,
    Freerider,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Strategy {
    RarestFirst,
    MostCommonFirst,
    Uniform,
}

#[derive(Debug)]
pub struct Config {
    number_chunks: usize,
    number_peers: usize,
    number_seeds: usize,
    peer_selfishness: Vec<Selfishness>,
    peer_strategies: Vec<Strategy>,
}

#[derive(Debug)]
pub struct Chunk {
    pub completion_round: Option<usize>,
    pub number_possessing_peers: usize,
}

#[derive(Debug)]
pub struct File {
    pub chunks: Vec<Chunk>,
}

#[derive(Debug)]
pub struct Peer {
    pub selfishness: Selfishness,
    pub strategy: Strategy,
    pub completion_round: Option<usize>,
    pub possessed_chunks: Vec<bool>,
    pub number_uploads: usize,
    current_target_peer: Option<usize>,
    pending_chunk: Option<usize>,
}

#[derive(Debug)]
pub struct Distribution {
    pub file: File,
    pub peers: Vec<Peer>,
    pub number_seeds: usize,
}

#[derive(Debug, Clone)]
pub struct Round {
    pub completed_peers: usize,
    pub completed_chunks: usize,
    pub exchanged_chunks: usize,
    pub execution_time: Duration,
}

pub trait RunObserver {
    fn random_seed(&self, _seed: u64) {}
    fn round_start(&self, _round_number: usize) {}
    fn chunk_transfer(&self, _chunk_number: usize, _source_peer: usize, _target_peer: usize) {}
    fn peer_completed(&self, _peer: usize) {}
    fn chunk_completed(&self, _chunk_number: usize) {}
    fn round_end(&self, _round_number: usize, _round: &Round) {}
}

pub struct EmptyRunObserver;
pub struct DebugRunObserver;
pub struct SummaryRunObserver;

impl Config {
    pub fn from_counts(
        number_chunks: usize,
        number_peers: usize,
        number_seeds: usize,
        number_selfish: usize,
        number_freeriders: usize,
        strategy: Strategy,
    ) -> Config {
        assert!(number_chunks > 0);
        assert!(number_seeds > 0);
        assert!(number_peers > number_seeds);
        assert!(number_seeds + number_selfish + number_freeriders <= number_peers);
        let mut selfishness =
            vec![Selfishness::Altruistic; number_peers - number_selfish - number_freeriders];
        selfishness.extend(vec![Selfishness::Selfish; number_selfish]);
        selfishness.extend(vec![Selfishness::Freerider; number_freeriders]);
        Config {
            number_chunks,
            number_peers,
            number_seeds,
            peer_selfishness: selfishness,
            peer_strategies: vec![strategy; number_peers],
        }
    }
}

impl Chunk {
    pub fn new(number_seeds: usize) -> Chunk {
        Chunk {
            completion_round: None,
            number_possessing_peers: number_seeds,
        }
    }
}

impl Peer {
    pub fn new(file: &File, is_seed: bool, selfishness: Selfishness, strategy: Strategy) -> Peer {
        assert!(!is_seed || selfishness == Selfishness::Altruistic);
        Peer {
            selfishness,
            strategy,
            completion_round: if is_seed { Some(0) } else { None },
            possessed_chunks: vec![is_seed; file.chunks.len()],
            number_uploads: 0,
            current_target_peer: None,
            pending_chunk: None,
        }
    }

    fn can_transfer_chunk(&self, chunk_number: usize) -> bool {
        let allows_download = self.selfishness == Selfishness::Altruistic || (self.selfishness == Selfishness::Selfish && self.completion_round.is_none());
        let has_chunk = self.possessed_chunks[chunk_number] && self.pending_chunk.map_or(true, |c| c != chunk_number);
        let has_capacity = self.current_target_peer.is_none();
        allows_download && has_chunk && has_capacity
    }
}

impl Distribution {
    pub fn new(config: &Config) -> Distribution {
        let mut chunks = Vec::with_capacity(config.number_chunks);
        for _ in 0..config.number_chunks {
            chunks.push(Chunk::new(config.number_seeds))
        }
        let file = File { chunks };
        let mut peers = Vec::with_capacity(config.number_peers);
        for i in 0..config.number_seeds {
            peers.push(Peer::new(&file, true, config.peer_selfishness[i], config.peer_strategies[i]))
        }
        for i in config.number_seeds..config.number_peers {
            peers.push(Peer::new(&file, false, config.peer_selfishness[i], config.peer_strategies[i]))
        }
        Distribution {
            file: file,
            peers: peers,
            number_seeds: config.number_seeds,
        }
    }

    pub fn run<Obs: RunObserver>(&mut self, random_seed: Option<u64>, observer: Obs) -> Vec<Round> {
        let random_seed = random_seed.unwrap_or(Utc::now().timestamp() as u64);
        observer.random_seed(random_seed);
        let mut rng = ChaCha8Rng::seed_from_u64(random_seed);
        let mut rounds = vec![];
        let mut current_round = Round {
            completed_peers: self.number_seeds,
            completed_chunks: 0,
            exchanged_chunks: 0,
            execution_time: Duration::from_secs(0),
        };
        rounds.push(current_round.clone());
        let mut shuffled_peers: Vec<usize> = (0..self.peers.len()).collect();
        let mut temporary_chunks: Vec<usize> = (0..self.file.chunks.len()).collect();
        let number_peers = self.peers.len();
        while current_round.completed_peers < self.peers.len() {
            observer.round_start(rounds.len());
            let start_time = Instant::now();
            let mut exchanged_chunks = 0;
            let mut completed_peers = 0;
            let mut completed_chunks = 0;
            shuffled_peers[0..self.number_seeds].shuffle(&mut rng);
            shuffled_peers[self.number_seeds..].shuffle(&mut rng);
            temporary_chunks.sort_by_key(|c| self.file.chunks[*c].number_possessing_peers);
            for peer_index in &shuffled_peers[self.number_seeds..number_peers] {
                if self.peers[*peer_index].completion_round.is_some() {
                    continue;
                }
                self.randomize_chunks(&mut rng, &mut temporary_chunks);
                let peer_chunks: Box<dyn Iterator<Item = &usize>> = match self.peers[*peer_index].strategy {
                    Strategy::RarestFirst => Box::new(temporary_chunks.iter()),
                    Strategy::MostCommonFirst => Box::new(temporary_chunks.iter().rev()),
                    Strategy::Uniform => Box::new(temporary_chunks.choose_multiple(&mut rng, temporary_chunks.len())),
                };
                'chunk_search: for chunk_index in peer_chunks {
                    if self.peers[*peer_index].possessed_chunks[*chunk_index] {
                        continue;
                    }
                    for shuffled_source_peer_index in
                        (self.number_seeds..number_peers).chain(0..self.number_seeds)
                    {
                        let source_peer_index = shuffled_peers[shuffled_source_peer_index];
                        if !self.peers[source_peer_index].can_transfer_chunk(*chunk_index) {
                            continue;
                        }
                        observer.chunk_transfer(*chunk_index, source_peer_index, *peer_index);
                        exchanged_chunks += 1;
                        let chunk = &mut self.file.chunks[*chunk_index];
                        chunk.number_possessing_peers += 1;
                        if chunk.number_possessing_peers == number_peers {
                            observer.chunk_completed(*chunk_index);
                            chunk.completion_round = Some(rounds.len());
                            completed_chunks += 1;
                        }
                        self.peers[source_peer_index].number_uploads += 1;
                        self.peers[source_peer_index].current_target_peer = Some(*peer_index);
                        self.peers[*peer_index].pending_chunk = Some(*chunk_index);
                        self.peers[*peer_index].possessed_chunks[*chunk_index] = true;
                        if self.peers[*peer_index].possessed_chunks.iter().all(|c| *c) {
                            observer.peer_completed(*peer_index);
                            self.peers[*peer_index].completion_round = Some(rounds.len());
                            completed_peers += 1;
                        }
                        break 'chunk_search;
                    }
                }
            }
            for peer in &mut self.peers {
                peer.current_target_peer = None;
                peer.pending_chunk = None;
            }
            current_round.completed_peers += completed_peers;
            current_round.completed_chunks += completed_chunks;
            current_round.exchanged_chunks = exchanged_chunks;
            current_round.execution_time = start_time.elapsed();
            observer.round_end(rounds.len(), &current_round);
            rounds.push(current_round.clone());
            current_round = Round::new(&current_round);
        }
        rounds
    }

    fn randomize_chunks<R: Rng + ?Sized>(&self, rng: &mut R, chunks: &mut Vec<usize>) {
        let mut i = 0;
        while i < chunks.len() - 1 {
            let chunk = &self.file.chunks[chunks[i]];
            let next_chunk = &self.file.chunks[chunks[i + 1]];
            if chunk.number_possessing_peers != next_chunk.number_possessing_peers {
                i += 1
            } else {
                let mut j = i + 2;
                while j < chunks.len() && self.file.chunks[chunks[j]].number_possessing_peers == chunk.number_possessing_peers {
                    j += 1
                }
                chunks[i..j].shuffle(rng);
                i = j;
            }
        }
    }
}

impl Round {
    pub fn new(previous_round: &Round) -> Round {
        Round {
            completed_peers: previous_round.completed_peers,
            completed_chunks: previous_round.completed_chunks,
            exchanged_chunks: 0,
            execution_time: Duration::from_secs(0),
        }
    }
}

impl RunObserver for EmptyRunObserver {}

impl RunObserver for DebugRunObserver {
    fn random_seed(&self, seed: u64) {
        println!("Random seed: {:?}", seed);
    }
    fn round_start(&self, round_number: usize) {
        println!("Start round {:?}", round_number);
    }
    fn chunk_transfer(&self, chunk_number: usize, source_peer: usize, target_peer: usize) {
        println!(
            "Transfer chunk {:?} from {:?} to {:?}",
            chunk_number, source_peer, target_peer
        );
    }
    fn peer_completed(&self, peer: usize) {
        println!("Peer {:?} completed", peer);
    }
    fn chunk_completed(&self, chunk_number: usize) {
        println!("Chunk {:?} fully distributed", chunk_number);
    }
    fn round_end(&self, round_number: usize, round: &Round) {
        println!(
            "End round {:?} time {:?}",
            round_number, round.execution_time
        );
    }
}

impl RunObserver for SummaryRunObserver {
    fn random_seed(&self, seed: u64) {
        println!("Random seed: {:?}", seed);
    }
    fn round_end(&self, round_number: usize, round: &Round) {
        println!("Round {:?}: {:?}", round_number, round);
    }
}
