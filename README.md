# POA ballot stats

A command line tool that displays voting statistics for the [POA network](https://poa.network/).
It and needs to communicate with a fully synchronized node that is connected to the network:
[POA installation](https://github.com/poanetwork/wiki/wiki/POA-Installation).
Note that `poa-ballot-stats` needs access to the network's full logs, so the node must run with
`--pruning=archive --no-warp`. 
Initial requirements for the tool described in RFC9 "Statistics of ballots." https://github.com/poanetwork/RFC/issues/9

# Usage

## Stable release

Download the archive for your platform from the latest [release](https://github.com/poanetwork/poa-ballot-stats/releases) and unpack it. Run the tool with `./poa-ballot-stats <options>`.

You can view the command line options with `-h`, and specify a different endpoint if your node e.g.
uses a non-standard port. By default, it tries to connect to a local node `http://127.0.0.1:8545`.

In verbose mode, with `-v`, the list of collected ballot and key change events is displayed, and for each ballot the list of participating and abstaining voters.

The `-c` option takes a map with the POA contracts' addresses in JSON format. You can find the 
current maps for the main and test network the `contracts` folder. By default, it uses `core.json`,
for the main network.

The `-p` option takes a time interval in hours, days, months, etc. E.g. `-p "10 weeks"` will only count participation in ballots that were created within the last 10 weeks. Alternatively, instead of a _time_, you can specify the earliest block _number_ as a decimal integer with the `-b` option.

Examples:

```bash
$ ./poa-ballot-stats -h
$ ./poa-ballot-stats
$ ./poa-ballot-stats https://core.poa.network -v -p "10 weeks"
$ ./poa-ballot-stats -c contracts/sokol.json https://sokol.poa.network -v
```

## Latest code

If you have a recent version of [Rust](https://www.rust-lang.org/), you can clone this repository and use `cargo run --` instead of `./poa-ballot-stats` to compile and run the latest version of the code.

## Screenshot

![Screenshot](screenshot.png)
