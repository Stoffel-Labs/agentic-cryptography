# Ad-Space Auction for AI Agents (Stoffel MPC)

A privacy-first *sealed-bid* ad auction where one AI agent auctions ad space
inside its product to other AI agents. Built on [Stoffel](https://docs.stoffelmpc.com),
a privacy-first Multi-Party Computation (MPC) framework.

## The two privacy guarantees

The whole reason this is an MPC app and not a normal backend:

1. **Bidders never learn each other's bids.** Each bidder is a separate MPC
   client slot. Its bid is sealed with `ClientStore.take_share(slot, 0)` the
   moment it is submitted, so the plaintext bid never leaves the bidder's own
   process. All bids stay secret-shared for the whole auction; the winner is
   found by a secret-shared **argmax** (secure comparison + oblivious select),
   which means no bid is ever opened mid-auction. A bidder cannot infer another
   bidder's value.

2. **The winner is hidden until the auction closes.** Nothing is revealed until
   the very end:
   - each bidder learns **only a single bit**: *did I win?* — not the price,
     not the other bids, not who else bid;
   - the auctioneer (host) learns **only the winner index and the winning
     price** — never the losing bids.

This satisfies the brief: *"you don't want this AI agent to know the winner
until the very end, and you don't want the bidders to know each other's bids."*

## How it maps to Stoffel

| Auction concept        | Stoffel mechanism                                            |
|------------------------|-------------------------------------------------------------|
| Bidding agent          | MPC client slot (`ClientStore.take_share(i, 0)`)           |
| Sealed bid             | `secret int64` — plaintext never leaves the bidder process  |
| Find winner            | secret argmax: `less_than` + `select` over shared bids       |
| Keep bids hidden       | bids stay secret-shared; no `open()` mid-auction            |
| Reveal winner only at end | `send_to_client(i, won_bit)` + host `open()` at the end  |

## Project layout

```
ad-auction-agent/
  Stoffel.toml              # project + MPC config (honeybadger, 5 parties, t=1)
  Cargo.toml               # Rust SDK wrapper + stoffel-bindgen (build dep)
  build.rs                 # generates typed client IO bindings from src/auction.stfl
  src/
    auction.stfl           # the MPC program (sealed-bid auction core)
    main.rs                # participant/auctioneer harness (runs local MPC)
  tests/
    local_mpc.rs           # integration test: winner/price correct + each bidder gets 1 won-bit
  auction/
    bids.json              # fixture: 4 sealed AI-agent bids (300, 500, 900, 700)
  target/debug/auction.stflb   # compiled bytecode (after `stoffel build`)
```

## Prerequisites

- Rust stable + Cargo
- The `stoffel` CLI: `curl -fsSL https://get.stoffelmpc.com | sh`
- `export PATH="$HOME/.local/bin:$PATH"`

## Run it

### 1. CLI smoke (local MPC, real secret shares)

```sh
stoffel status --verbose
stoffel check
stoffel build                       # -> target/debug/auction.stflb
stoffel run \
  --client-input-file auction/bids.json \
  --expected-output-clients 4 \
  --program-info --timeout-secs 250
```

Expected host output: `[2, 900]` (winner = agent slot 2, price = 900).
Each bidder also receives its `won` bit via the client-output channel; no
bid value is ever printed to any bidder or to the host.

You can also pass bids inline:

```sh
stoffel run --client-input 0=300 --client-input 1=500 \
  --client-input 2=900 --client-input 3=700 \
  --expected-output-clients 4 --timeout-secs 250
```

### 2. Rust participant/auctioneer harness

```sh
cargo build --locked
cargo run --locked --quiet
```

Prints, e.g.:

```
=== Auctioneer (host) sees ===
  winning agent slot : 2
  winning price      : 900
  (every losing bid stays secret -- the host never sees them)
=== Each bidding agent learns (and nothing else) ===
  agent slot 0 : won = false
  agent slot 1 : won = false
  agent slot 2 : won = true
  agent slot 3 : won = false
```

### 3. Integration test

```sh
cargo test --locked
```

Asserts the winner/price are correct AND that each bidder receives exactly one
output value (its won-bit) — proving the privacy boundary.

## Customizing

- **Number of bidders** (`n`): edit `n` in `src/auction.stfl` and pass that many
  `--client-input` / fixture slots.
- **Bid range** (`l`): bids must be in `[0, 2^l)`. `l = 20` allows ~1,048,575
  units; raise it for larger price spaces.
- **Masking security** (`kappa`): `8` is fine for local dev; production should
  use `~40`.
- **Pricing rule**: this app uses **first-price** (winner pays its own bid).
  Switch to **second-price / Vickrey** (winner pays the second-highest bid) by
  sorting the bids in MPC and returning `sorted[n-2]` as the price — see the
  framework's `mpc_second_price_auction` example for the pattern.

## Privacy / deployment notes

- The local run spawns a real 5-party honeybadger network on your machine and
  one trusted harness process sees every fixture input. It proves the *program
  semantics*, not that a production application service is outside the plaintext
  path.
- For production, each bidding AI agent runs its **own** Stoffel client that
  loads the pinned bytecode (`target/debug/auction.stflb`), generates typed
  bindings from it, and submits its sealed bid directly to the separately
  deployed MPC service. The auctioneer control plane must not receive or persist
  any bid plaintext. Use `cargo build --release` and deploy the bytecode +
  bindings to each participant.
- Regenerate bindings after any change to `src/auction.stfl` (the `build.rs`
  script does this automatically on `cargo build`).
