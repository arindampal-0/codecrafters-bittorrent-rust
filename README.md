# Build your own BitTorrent

[CodeCrafters](https://app.codecrafters.io)

[CodeCrafters Build your own BitTorrent](https://app.codecrafters.io/courses/bittorrent)

Programming Language used: **Rust**

## Setup

To build the project
```shell
cargo build
```

### Different commands

Decode
```shell
$ cargo run decode 5:hello
"hello"
```

Info
```shell
$ cargo run info sample.torrent
Tracker URL: http://bittorrent-test-tracker.codecrafters.io/announce
Length: 92063
Info Hash: d69f91e6b2ae4c542468d1073a71d4ea13879a7f
Piece Length: 32768
Piece Hashes:
e876f67a2a8886e8f36b136726c30fa29703022d
6e2275e604a0766656736e81ff10b55204ad8d35
f00d937a0213df1982bc8d097227ad9e909acc17
```

Peers
```shell
$ cargo run peers sample.torrent
178.62.82.89:51470
165.232.33.77:51467
178.62.85.20:51489
```

Handshake
```shell
$ cargo run handshake sample.torrent 165.232.33.77:51467
Peer ID: 2d524e302e302e302d5af5c2cf488815c4a2fa7f
```

Download Piece
```shell
cargo run download_piece -o tmp/test-piece-0 sample.torrent 0
```

Download the whole file
```shell
cargo run download -o tmp/test.txt sample.torrent
```

## CodeCrafters Instructions

[![progress-banner](https://backend.codecrafters.io/progress/bittorrent/7fbfe379-a889-4f80-94c6-cf92d83c69fc)](https://app.codecrafters.io/users/ArindamPal-0?r=2qF)

This is a starting point for Rust solutions to the
["Build Your Own BitTorrent" Challenge](https://app.codecrafters.io/courses/bittorrent/overview).

In this challenge, you’ll build a BitTorrent client that's capable of parsing a
.torrent file and downloading a file from a peer. Along the way, we’ll learn
about how torrent files are structured, HTTP trackers, BitTorrent’s Peer
Protocol, pipelining and more.

**Note**: If you're viewing this repo on GitHub, head over to
[codecrafters.io](https://codecrafters.io) to try the challenge.

### Passing the first stage

The entry point for your BitTorrent implementation is in `src/main.rs`. Study
and uncomment the relevant code, and push your changes to pass the first stage:

```sh
git add .
git commit -m "pass 1st stage" # any msg
git push origin master
```

Time to move on to the next stage!

### Stage 2 & beyond

Note: This section is for stages 2 and beyond.

1. Ensure you have `cargo (1.70)` installed locally
1. Run `./your_bittorrent.sh` to run your program, which is implemented in
   `src/main.rs`. This command compiles your Rust project, so it might be slow
   the first time you run it. Subsequent runs will be fast.
1. Commit your changes and run `git push origin master` to submit your solution
   to CodeCrafters. Test output will be streamed to your terminal.
