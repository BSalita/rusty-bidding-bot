# Contributing

## Clean room: read this first

This project is an independent implementation. It is **not** derived from
BBA / EPBot.

- **Do not** read, quote, or model on the decompiled EPBot source, or any BBA
  source code. If you have studied it, say so in your pull request and do not
  work on the areas you studied.
- **Fine:** running `bba-cli` or bba-server to produce reference auctions,
  comparing our auctions against them, and reading `.bbsa` files in order to
  import them.
- Rules should come from bridge literature, published convention
  descriptions, the convention card, and observed bidding.

## Adding a convention

Conventions, the card settings that turn them on and their rule files are
in [bridge-common-conventions](https://github.com/bridge-craftwork/bridge-common-conventions), not here. A
convention is a pull request there:

1. Add or find its definition and the card setting that turns it on.
2. Write a `.bid` module (syntax: docs/LANGUAGE.md). Every rule needs an
   explanation and a `shows` clause.
3. Add test auctions (a `.test` file beside the module), and check the corpus
   scoreboard for regressions: `rbb bid check --rules .` and
   `rbb bid test --rules .` in a checkout of that repo, `rbb compare --rules
   <checkout>` here.

This repo takes the result at a release, when `scripts/sync_pack.py`
refreshes `pack/pack.json`.

docs/CONTRACT.md lists what the engine promises a rule set and what a good
convention file does (tests, notes, measuring a change).

## License

Contributions are dual licensed MIT OR Apache-2.0, as described in README.md.
