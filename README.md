# Coppa

Coppa is a simulator for a Bittorrent-like peer-to-peer content distribution
system, where the file to be distributed is split into chunks, and chunks are
downloaded independently of each other. For any chunk, the downloading peers
who already possess that chunk are also potential sources of it.

This is still a simple version that is missing a lot of functionality in the
simulation. Peer behavior will be much more configurable in future versions.
