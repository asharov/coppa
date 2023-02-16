# Coppa

Coppa is a simulator for a Bittorrent-like peer-to-peer content distribution
system, where the file to be distributed is split into chunks, and chunks are
downloaded independently of each other. For any chunk, the downloading peers
who already possess that chunk are also potential sources of it.

Coppa is primarily a library that can be used to program different kinds of
applications that need a simulation of content distribution. It also comes
with a program that runs a simulation with everything about the simulation
being configurable.

## Running

Coppa can be run from the source directory with `cargo run --release` (use
the `--release` flag, as a debug run for a large distribution is extremely
slow). The most basic run is
```bash
cargo run --release -- --chunks=20 --peers=16
```
Here, the file to be distributed is split into 20 chunks, and there are a
total of 16 peers, one of which is the seed possessing the whole file in the
beginning and 15 are peers who wish to download the file.

The output provides a summary of the distribution by printing the statistics
of each round. In this simple case, a round is defined as the time it takes
to download one chunk. The output of a round looks something like
```
Round 17: Round { completed_peers: 1, completed_chunks: 6, exchanged_chunks: 15, execution_time: 17.066µs }
```
with the variables being
- completed_peers: The number of peers who have downloaded the whole file at
the end of the round. This includes the seeds, so that this number is always
at least the number of seeds.
- completed_chunks: The number of chunks that are possessed by all peers at
the end of the round.
- exchanged_chunks: The number of chunks that were downloaded during that
round. For optimal distribution performance, this should be close to the
number of non-seed peers.
- execution_time: The time it took to simulate the round.

To get a detailed output of everything that happens during the simulation,
pass the option `--verbose`. To suppress all per-round output and only
output the summary at the end, pass the `--silent` option.

When `--silent` is not used, the random seed used for the run is also printed.
This can be used to replicate the same run in the future by passing it with
the `--random-seed` option.

`cargo run -- --help` gives a summary of all the options.

### Selfishness

The default assumption is that a peer participating in the distribution
is willing to upload any chunks that it possesses and stays available until
all peers possess the whole file. These assumptions can be broken with the
following peer behaviors
- Altruistic: This is the default behavior
- Selfish: The peer is willing to upload chunks, but it will not upload
anything after it possesses the whole file. This is a peer that leaves the
distribution network after its download is completed.
- Freerider: The peer never uploads chunks, only downloads.

### Chunk Selection Strategy

A peer can select the next chunk to download in multiple ways. These are
the available strategies:
- Rarest First: The peer selects the chunk that is possessed by the smallest
number of peers in the network.
- Most Common First: The peer selects the chunk that is possessed by the
largest number of peers in the network.
- Uniform: The peer picks randomly with a uniform distribution.
In both Rarest First and Most Common First, if there are multiple candidate
chunks possessed by the same number of peers, the chunk to download will be
selected randomly from them.

### Peer Network Speed

Peers do not necessarily have equally fast network connections. For
simplicity, the available speeds are divided into three tiers, Fast, Medium,
and Slow. The tier of each peer and the actual speed of each tier are
configurable. If different speed tiers are configured, the definition of
a round from earlier is no longer accurate, as downloading a chunk can now
take multiple rounds.

Currently, a fast peer can upload chunks to multiple slow peers, but a fast
peer cannot download from multiple slow peers during a single round. This
may be developed further at some point.

### Peer Configuration

Simple peer behavior can be controlled with the `--selfish`, `--freerider`,
and `--strategy` options. `--selfish` and `--freerider` give the number of
Selfish and Freerider peers, respectively, and `--strategy` gives the chunk
selection strategy used by all peers. When using these simple options, all
peers are assumed to be fast.

More complicated configuration is done by supplying a file name to the
`--peer-config-file` option. This file is a text file, with each line
describing a peer in a short format. If there are more non-seed peers than
lines in this file, the remaining peers are set to the default of
Altruistic, Rarest First, Fast. This is also always the seed configuration.
(Though a seed's chunk selection strategy does not matter, as a seed will
never download chunks.)

A line in this file consists of three letters, in the order of Selfishness,
Strategy, Speed. Each letter is the lowercase first letter of the corresponding
behavior. For instance
- `srf` is Selfish, Rarest First, Fast
- `fmm` is Freerider, Most Common First, Medium
- `aus` is Altruistic, Uniform, Slow

## Library

The library documentation is still non-existent. Here are some basics.

When using Coppa as a library, the important types are `Config` and `Distribution`.
A `Config` object needs to be constructed to be passed to `Distribution::new`.
When configuring peers in more detail, `Config::from_peer_config` also requires
`PeerConfig` objects for the peers.

A `Distribution` can be simulated with `run()`. This takes an `Observer`
argument that can be used to monitor the progress of the distribution. A
`Distribution` should leave its internal state clean after a `run()`, so
repeating a simulation with another call to `run()` should be possible.

Other types in the library are not interesting to a user of the library,
and the publicness of the types and their fields is likely to change in the
future.
