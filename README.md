# POA ballot stats

**Note**: This is still work in progress. It doesn't yet correctly determine the initial set of
validators.

A command line tool that displays voting statistics for the [POA network](https://poa.network/).
It requires a recent version of [Rust](https://www.rust-lang.org/), and needs to communicate with a
fully synchronized node that is connected to the network:
[POA installation](https://github.com/poanetwork/wiki/wiki/POA-Installation).

You can view the command line options with `-h`, and specify a different endpoint if your node e.g.
uses a non-standard port. The `-c` option takes a map with the POA contracts' addresses in JSON
format. You can find the current maps for the main and test network in
[poa-chain-spec](https://github.com/poanetwork/poa-chain-spec)'s `core` and `sokol` branches.

```bash
$ cargo run -- -h
$ cargo run -- -c ../poa-chain-spec/contracts.json http://127.0.0.1:8545
```

