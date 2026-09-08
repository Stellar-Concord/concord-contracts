# concord-escrow

A permissionless, milestone-based escrow contract for Stellar/Soroban. Any
platform (marketplaces, freelance platforms, service marketplaces) can
integrate escrow into its payment flow using this contract directly — funds
are held trustlessly on-chain, never by a custodian, and disputes are
resolved through a defined on-chain arbitration flow rather than off-chain
negotiation.

Part of the Concord project, alongside [`concord-backend`](../concord-backend)
(an event indexer + REST API) and [`concord-frontend`](../concord-frontend)
(a reference UI). This repo is the source of truth: the backend and frontend
both read the state this contract defines and emits.

## How it works

An escrow has one client, one provider, one arbitrator (all distinct — see
[Design notes](#design-notes)), a settlement token (any SEP-41-compliant
token, including the Stellar Asset Contract), and one or more milestones,
each with its own amount.

```
Created ──fund──▶ Funded ──submit──▶ InProgress ──approve (last milestone)──▶ Completed
   │                                     │
 cancel                              raise_dispute
   │                                     │
   ▼                                     ▼
Cancelled                    Disputed (per milestone) ──resolve──▶ back to InProgress
                                                                    or Completed if that
                                                                    was the last one
```

- The **client** funds the escrow (transfers the full sum up front) and
  approves each milestone as the provider delivers it, releasing that
  milestone's funds.
- The **provider** submits a milestone as delivered, then waits for
  approval.
- Either the **client or provider** can dispute a milestone that hasn't been
  released yet. This freezes only that milestone — the rest of the escrow
  keeps moving normally.
- The **arbitrator** resolves a dispute by releasing the milestone's funds
  fully to the provider, fully back to the client, or split between them by
  basis points (e.g. `Split(6000)` = 60% provider / 40% client).
- The escrow completes automatically once every milestone is either
  approved or dispute-resolved.

## Contract interface

| Function | Who | What |
|---|---|---|
| `initialize_escrow(client, provider, arbitrator, token, milestone_descriptions, milestone_amounts) -> u64` | client | Creates the escrow, returns its id |
| `fund_escrow(escrow_id)` | client | Transfers the full sum into the contract |
| `submit_milestone(escrow_id, milestone_id)` | provider | Marks a milestone delivered |
| `approve_milestone(escrow_id, milestone_id)` | client | Releases that milestone's funds to the provider |
| `cancel_escrow(escrow_id)` | client | Cancels before funding (only) |
| `raise_dispute(escrow_id, milestone_id, raised_by, reason)` | client or provider | Freezes a milestone pending arbitration |
| `resolve_dispute(escrow_id, milestone_id, resolution)` | arbitrator | Settles a disputed milestone |
| `get_escrow(escrow_id) -> Escrow` | anyone | Reads current state |
| `get_dispute(escrow_id, milestone_id) -> Dispute` | anyone | Reads a dispute's raiser/reason |
| `get_dispute_resolution(escrow_id, milestone_id) -> Resolution` | anyone | Reads how a resolved dispute was settled |

`Resolution` is `ReleaseToProvider`, `RefundToClient`, or `Split(u32)` (basis
points, 0–10000, to the provider).

Every state transition publishes a typed event (`EscrowCreated`,
`EscrowFunded`, `MilestoneSubmitted`, `MilestoneApproved`, `EscrowCancelled`,
`EscrowCompleted`, `DisputeRaised`, `DisputeResolved`) — this is what
`concord-backend`'s indexer consumes to reconstruct state off-chain.

## Design notes

- **Client, provider, and arbitrator must be distinct addresses.**
  `initialize_escrow` rejects any overlap — otherwise, e.g., an arbitrator
  who is also the client could dispute their own milestone and resolve it
  in their own favor.
- **State is updated before any external token call.** `fund_escrow`,
  `approve_milestone`, and `resolve_dispute` all persist the new
  status before calling the (caller-supplied) token contract's `transfer`.
  The token address is chosen by whoever creates the escrow, so a
  malicious or buggy token could otherwise call back into the same
  function mid-transfer and get paid twice for the same milestone.
- **Every write extends TTL.** Persistent storage entries and the contract
  instance itself both get their TTL bumped on every state change, so an
  escrow that sits untouched for a long stretch between milestones doesn't
  risk archival.
- **Not upgradable.** There's no admin key that can swap the deployed wasm.
  Simpler trust model (nothing can rug the funds it holds), at the cost of
  needing a new deployment to fix a bug or add a feature.

## Project layout

```
contracts/escrow/src/
├── lib.rs        # #[contract] struct + module wiring only
├── escrow.rs     # initialize_escrow, fund_escrow, cancel_escrow, get_escrow
├── milestone.rs  # submit_milestone, approve_milestone
├── dispute.rs    # raise_dispute, resolve_dispute, get_dispute*
├── state.rs      # storage access -- the only place that touches env.storage()
├── types.rs      # Escrow, Milestone, Dispute, Resolution, status enums
├── events.rs     # #[contractevent] definitions
├── errors.rs     # contract error codes
└── test/         # unit + property tests, split to mirror the modules above
```

`escrow.rs`, `milestone.rs`, and `dispute.rs` each have their own
`#[contractimpl]` block for the same `EscrowContract` type — Soroban merges
multiple such blocks into one exported contract as long as function names
don't collide, so the domains stay in separate files instead of one large
`lib.rs`.

## Building

Requires the `wasm32v1-none` Rust target and the [Stellar
CLI](https://developer.stellar.org/docs/tools/cli/install-cli).

```bash
rustup target add wasm32v1-none
stellar contract build --out-dir target/wasm-out
```

## Testing

```bash
cargo test -p concord-escrow
```

32 tests: example-based unit tests for the full lifecycle and every
rejection path, a set that specifically proves `require_auth()` rejects the
wrong signer (everything else uses `mock_all_auths()`, which wouldn't catch
that on its own), and [proptest](https://proptest-rs.github.io/proptest/)
property tests on the split/amount math across a wide input range rather
than a handful of hand-picked cases.

```bash
cargo fmt -p concord-escrow -- --check
cargo clippy -p concord-escrow --all-targets -- -D warnings
```

CI (`.github/workflows/ci.yml`) runs all of the above plus a wasm build
check on every push and PR.

## Deployment

See [`DEPLOYMENTS.md`](./DEPLOYMENTS.md) for the current testnet contract
address and what's been verified against it live.

```bash
stellar keys generate <your-key-name> -n testnet --fund
stellar contract deploy \
  --wasm target/wasm-out/concord_escrow.wasm \
  --source <your-key-name> -n testnet --alias concord_escrow
```
