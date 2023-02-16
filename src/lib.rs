use chrono::Utc;
use clap::ValueEnum;
use num::integer::{gcd, lcm};
use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::cmp;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Selfishness {
    #[default]
    Altruistic,
    Selfish,
    Freerider,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, ValueEnum)]
pub enum Strategy {
    #[default]
    RarestFirst,
    MostCommonFirst,
    Uniform,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Speed {
    #[default]
    Fast,
    Medium,
    Slow,
}

#[derive(Debug)]
pub struct PeerConfig {
    selfishness: Selfishness,
    strategy: Strategy,
    speed: Speed,
}

#[derive(Debug)]
pub struct Config {
    number_chunks: usize,
    number_peers: usize,
    number_seeds: usize,
    chunk_size: usize,
    peer_selfishness: Vec<Selfishness>,
    peer_strategies: Vec<Strategy>,
    peer_speeds: Vec<usize>,
}

#[derive(Debug)]
pub struct Chunk {
    index: usize,
    pub completion_round: Option<usize>,
    pub number_possessing_peers: usize,
}

#[derive(Debug)]
pub struct File {
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, Copy)]
struct Download {
    chunk_number: usize,
    source_peer: usize,
    target_peer: usize,
    downloaded_size: usize,
    current_size: usize,
}

#[derive(Debug)]
pub struct Peer {
    index: usize,
    pub selfishness: Selfishness,
    pub strategy: Strategy,
    pub speed: usize,
    pub completion_round: Option<usize>,
    pub possessed_chunks: Vec<bool>,
    pub number_uploads: usize,
    current_uploads: Vec<Download>,
    current_download: Option<Download>,
}

#[derive(Debug)]
pub struct Distribution {
    pub file: File,
    pub peers: Vec<Peer>,
    pub number_seeds: usize,
    chunk_size: usize,
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
    fn chunk_size(&self, _chunk_size: usize) {}
    fn round_start(&self, _round_number: usize) {}
    fn chunk_transfer(
        &self,
        _chunk_number: usize,
        _transfer_size: usize,
        _source_peer: usize,
        _target_peer: usize,
    ) {
    }
    fn peer_completed(&self, _peer: usize) {}
    fn chunk_completed(&self, _chunk_number: usize) {}
    fn round_end(&self, _round_number: usize, _round: &Round) {}
}

pub struct EmptyRunObserver;
pub struct DebugRunObserver;
pub struct SummaryRunObserver;

impl PeerConfig {
    pub fn from_string(config_string: &[u8]) -> PeerConfig {
        let selfishness = match config_string.first().unwrap_or(&b'a') {
            b's' => Selfishness::Selfish,
            b'f' => Selfishness::Freerider,
            _ => Selfishness::Altruistic,
        };
        let strategy = match config_string.get(1).unwrap_or(&b'r') {
            b'm' => Strategy::MostCommonFirst,
            b'u' => Strategy::Uniform,
            _ => Strategy::RarestFirst,
        };
        let speed = match config_string.get(2).unwrap_or(&b'f') {
            b'm' => Speed::Medium,
            b's' => Speed::Slow,
            _ => Speed::Fast,
        };
        PeerConfig {
            selfishness,
            strategy,
            speed,
        }
    }
}

impl Config {
    fn assert_common_parameters(
        number_chunks: usize,
        number_peers: usize,
        number_seeds: usize,
        speed_fast: usize,
        speed_medium: usize,
        speed_slow: usize,
    ) {
        assert!(number_chunks > 0);
        assert!(number_seeds > 0);
        assert!(number_peers > number_seeds);
        assert!(speed_slow > 0);
        assert!(speed_medium >= speed_slow);
        assert!(speed_fast >= speed_medium);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_counts(
        number_chunks: usize,
        number_peers: usize,
        number_seeds: usize,
        speed_fast: usize,
        speed_medium: usize,
        speed_slow: usize,
        number_selfish: usize,
        number_freeriders: usize,
        strategy: Strategy,
    ) -> Config {
        Self::assert_common_parameters(
            number_chunks,
            number_peers,
            number_seeds,
            speed_fast,
            speed_medium,
            speed_slow,
        );
        assert!(number_seeds + number_selfish + number_freeriders <= number_peers);
        let speed_gcd = gcd(gcd(speed_slow, speed_medium), speed_fast);
        let speed_fast = speed_fast / speed_gcd;
        let speed_medium = speed_medium / speed_gcd;
        let speed_slow = speed_slow / speed_gcd;
        assert!(speed_fast <= 1000);
        let chunk_size = lcm(lcm(speed_slow, speed_medium), speed_fast);
        let mut selfishness =
            vec![Selfishness::Altruistic; number_peers - number_selfish - number_freeriders];
        selfishness.extend(vec![Selfishness::Selfish; number_selfish]);
        selfishness.extend(vec![Selfishness::Freerider; number_freeriders]);
        Config {
            number_chunks,
            number_peers,
            number_seeds,
            chunk_size,
            peer_selfishness: selfishness,
            peer_strategies: vec![strategy; number_peers],
            peer_speeds: vec![speed_fast / speed_gcd; number_peers],
        }
    }

    pub fn from_peer_config(
        number_chunks: usize,
        number_peers: usize,
        number_seeds: usize,
        speed_fast: usize,
        speed_medium: usize,
        speed_slow: usize,
        peer_config: Vec<PeerConfig>,
    ) -> Config {
        Self::assert_common_parameters(
            number_chunks,
            number_peers,
            number_seeds,
            speed_fast,
            speed_medium,
            speed_slow,
        );
        assert!(peer_config.len() <= number_peers - number_seeds);
        let speed_gcd = gcd(gcd(speed_slow, speed_medium), speed_fast);
        let speed_fast = speed_fast / speed_gcd;
        let speed_medium = speed_medium / speed_gcd;
        let speed_slow = speed_slow / speed_gcd;
        assert!(speed_fast <= 1000);
        let chunk_size = lcm(lcm(speed_slow, speed_medium), speed_fast);
        let mut peer_selfishness = vec![Selfishness::Altruistic; number_seeds];
        peer_selfishness.extend(peer_config.iter().map(|c| c.selfishness));
        peer_selfishness.extend(vec![
            Selfishness::default();
            number_peers - peer_selfishness.len()
        ]);
        let mut peer_strategies = vec![Strategy::default(); number_seeds];
        peer_strategies.extend(peer_config.iter().map(|c| c.strategy));
        peer_strategies.extend(vec![
            Strategy::default();
            number_peers - peer_strategies.len()
        ]);
        let mut peer_speeds = vec![Speed::Fast; number_seeds];
        peer_speeds.extend(peer_config.iter().map(|c| c.speed));
        peer_speeds.extend(vec![Speed::default(); number_peers - peer_speeds.len()]);
        let peer_speeds = peer_speeds
            .iter()
            .map(|s| match s {
                Speed::Fast => speed_fast,
                Speed::Medium => speed_medium,
                Speed::Slow => speed_slow,
            })
            .collect();
        Config {
            number_chunks,
            number_peers,
            number_seeds,
            chunk_size,
            peer_selfishness,
            peer_strategies,
            peer_speeds,
        }
    }
}

impl Chunk {
    pub fn new(index: usize, number_seeds: usize) -> Chunk {
        Chunk {
            index,
            completion_round: None,
            number_possessing_peers: number_seeds,
        }
    }
}

impl Peer {
    pub fn new(
        index: usize,
        file: &File,
        is_seed: bool,
        selfishness: Selfishness,
        strategy: Strategy,
        speed: usize,
    ) -> Peer {
        assert!(!is_seed || selfishness == Selfishness::Altruistic);
        Peer {
            index,
            selfishness,
            strategy,
            speed,
            completion_round: if is_seed { Some(0) } else { None },
            possessed_chunks: vec![is_seed; file.chunks.len()],
            number_uploads: 0,
            current_uploads: vec![],
            current_download: None,
        }
    }

    fn available_capacity_for_chunk(&self, chunk_number: usize, target_peer: usize) -> usize {
        let allows_download = self.selfishness == Selfishness::Altruistic
            || (self.selfishness == Selfishness::Selfish && self.completion_round.is_none());
        let has_chunk = self.possessed_chunks[chunk_number];
        if allows_download && has_chunk {
            let used_capacity: usize = self
                .current_uploads
                .iter()
                .filter_map(|u| {
                    if u.target_peer == target_peer {
                        None
                    } else {
                        Some(u.current_size)
                    }
                })
                .sum();
            if used_capacity > self.speed {
                0
            } else {
                self.speed - used_capacity
            }
        } else {
            0
        }
    }

    fn index_of_upload(&self, chunk_number: usize, target_peer: usize) -> Option<usize> {
        self.current_uploads
            .iter()
            .position(|u| u.chunk_number == chunk_number && u.target_peer == target_peer)
    }

    fn download(&mut self, download: Download) {
        if let Some(index) = self.index_of_upload(download.chunk_number, download.target_peer) {
            self.current_uploads[index] = download
        } else {
            self.current_uploads.push(download)
        }
    }

    fn chunk_upload_finished(&mut self, chunk_number: usize, target_peer: usize) {
        if let Some(index) = self.index_of_upload(chunk_number, target_peer) {
            self.number_uploads += 1;
            self.current_uploads.remove(index);
        }
    }

    fn check_chunk_download_finished(&mut self, chunk_size: usize) -> Option<Download> {
        if let Some(download) = self.current_download {
            if download.downloaded_size >= chunk_size {
                self.possessed_chunks[download.chunk_number] = true;
                self.current_download = None;
                Some(download)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Distribution {
    pub fn new(config: &Config) -> Distribution {
        let mut chunks = Vec::with_capacity(config.number_chunks);
        for i in 0..config.number_chunks {
            chunks.push(Chunk::new(i, config.number_seeds))
        }
        let file = File { chunks };
        let mut peers = Vec::with_capacity(config.number_peers);
        for i in 0..config.number_seeds {
            peers.push(Peer::new(
                i,
                &file,
                true,
                config.peer_selfishness[i],
                config.peer_strategies[i],
                config.peer_speeds[i],
            ))
        }
        for i in config.number_seeds..config.number_peers {
            peers.push(Peer::new(
                i,
                &file,
                false,
                config.peer_selfishness[i],
                config.peer_strategies[i],
                config.peer_speeds[i],
            ))
        }
        Distribution {
            file,
            peers,
            number_seeds: config.number_seeds,
            chunk_size: config.chunk_size,
        }
    }

    pub fn run<Obs: RunObserver>(&mut self, random_seed: Option<u64>, observer: Obs) -> Vec<Round> {
        let random_seed = random_seed.unwrap_or(Utc::now().timestamp() as u64);
        observer.random_seed(random_seed);
        let mut rng = ChaCha8Rng::seed_from_u64(random_seed);
        observer.chunk_size(self.chunk_size);
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
                if let Some(mut download) = self.peers[*peer_index].current_download {
                    let desired_capacity = self.desired_download_capacity(
                        download.chunk_number,
                        download.source_peer,
                        *peer_index,
                    );
                    let remaining_size = self.chunk_size - download.downloaded_size;
                    let desired_size = cmp::min(desired_capacity, remaining_size);
                    assert!(desired_size > 0);
                    observer.chunk_transfer(
                        download.chunk_number,
                        desired_size,
                        download.source_peer,
                        download.target_peer,
                    );
                    download.current_size = desired_size;
                    download.downloaded_size += desired_size;
                    self.peers[*peer_index].current_download = Some(download);
                    self.peers[download.source_peer].download(download);
                    continue;
                }
                self.randomize_chunks(&mut rng, &mut temporary_chunks);
                let peer_chunks: Box<dyn Iterator<Item = &usize>> =
                    match self.peers[*peer_index].strategy {
                        Strategy::RarestFirst => Box::new(temporary_chunks.iter()),
                        Strategy::MostCommonFirst => Box::new(temporary_chunks.iter().rev()),
                        Strategy::Uniform => Box::new(
                            temporary_chunks.choose_multiple(&mut rng, temporary_chunks.len()),
                        ),
                    };
                'chunk_search: for chunk_index in peer_chunks {
                    if self.peers[*peer_index].possessed_chunks[*chunk_index] {
                        continue;
                    }
                    for shuffled_source_peer_index in
                        (self.number_seeds..number_peers).chain(0..self.number_seeds)
                    {
                        let source_peer_index = shuffled_peers[shuffled_source_peer_index];
                        let desired_capacity = self.desired_download_capacity(
                            *chunk_index,
                            source_peer_index,
                            *peer_index,
                        );
                        if desired_capacity == 0 {
                            continue;
                        }
                        observer.chunk_transfer(
                            *chunk_index,
                            desired_capacity,
                            source_peer_index,
                            *peer_index,
                        );
                        exchanged_chunks += 1;
                        let download = Download {
                            chunk_number: *chunk_index,
                            source_peer: source_peer_index,
                            target_peer: *peer_index,
                            downloaded_size: desired_capacity,
                            current_size: desired_capacity,
                        };
                        self.peers[*peer_index].current_download = Some(download);
                        self.peers[source_peer_index].download(download);
                        break 'chunk_search;
                    }
                }
            }
            let mut finished_uploads: Vec<Download> = vec![];
            for peer in &mut self.peers {
                if let Some(download) = peer.check_chunk_download_finished(self.chunk_size) {
                    let chunk = &mut self.file.chunks[download.chunk_number];
                    chunk.number_possessing_peers += 1;
                    if chunk.number_possessing_peers == number_peers {
                        observer.chunk_completed(chunk.index);
                        chunk.completion_round = Some(rounds.len());
                        completed_chunks += 1;
                    }
                    if peer.possessed_chunks.iter().all(|c| *c) {
                        observer.peer_completed(peer.index);
                        peer.completion_round = Some(rounds.len());
                        completed_peers += 1;
                    }
                    finished_uploads.push(download)
                }
            }
            for upload in finished_uploads {
                self.peers[upload.source_peer]
                    .chunk_upload_finished(upload.chunk_number, upload.target_peer)
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
                while j < chunks.len()
                    && self.file.chunks[chunks[j]].number_possessing_peers
                        == chunk.number_possessing_peers
                {
                    j += 1
                }
                chunks[i..j].shuffle(rng);
                i = j;
            }
        }
    }

    fn desired_download_capacity(
        &self,
        chunk_number: usize,
        source_peer: usize,
        target_peer: usize,
    ) -> usize {
        let upload_capacity =
            self.peers[source_peer].available_capacity_for_chunk(chunk_number, target_peer);
        let target_speed = self.peers[target_peer].speed;
        cmp::min(target_speed, upload_capacity)
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
        println!("Random seed: {seed:?}");
    }
    fn chunk_size(&self, chunk_size: usize) {
        println!("Chunk size: {chunk_size:?}");
    }
    fn round_start(&self, round_number: usize) {
        println!("Start round {round_number:?}");
    }
    fn chunk_transfer(
        &self,
        chunk_number: usize,
        transfer_size: usize,
        source_peer: usize,
        target_peer: usize,
    ) {
        println!("Transfer size {transfer_size:?} of chunk {chunk_number:?} from {source_peer:?} to {target_peer:?}");
    }
    fn peer_completed(&self, peer: usize) {
        println!("Peer {peer:?} completed");
    }
    fn chunk_completed(&self, chunk_number: usize) {
        println!("Chunk {chunk_number:?} fully distributed");
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
        println!("Random seed: {seed:?}");
    }
    fn round_end(&self, round_number: usize, round: &Round) {
        println!("Round {round_number:?}: {round:?}");
    }
}
