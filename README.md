# POA ballot stats

**Note**: This is still work in progress. It doesn't yet correctly determine the initial set of
validators.

A command line tool that displays voting statistics for the [POA network](https://poa.network/).
It requires a recent version of [Rust](https://www.rust-lang.org/), and needs to communicate with a
fully synchronized node that is connected to the network:
[POA installation](https://github.com/poanetwork/wiki/wiki/POA-Installation).

With the default setup, it should work without any additional options. You can view the command line
options with `-h`, and specify a different endpoint if your node e.g. uses a non-standard port. 

```bash
$ cargo run
$ cargo run -- -h
$ cargo run -- http://127.0.0.1:8545
```

