# Deployments

## Testnet (current)

Deployed 2026-09-08. This is the hardened version — the reentrancy fix,
TTL extension, and role-distinctness check described below are all live
on this contract.

| | |
|---|---|
| Escrow contract | `CBEAFDUIFRS7DPDN7C7SGSAKNAMY4MK2KVJBW4YFKFS2OAEPKUHJFLRL` |
| Wasm hash | `eebf50503adea96425a9e0a0efbbbf6f9a7012b9bc11f62b03295b7a658fb122` |
| Test token (SAC over classic asset `CORD`) | `CCCXMQ2ZY4CPSDO3TX2AHIEEEJG6CV7QUJ237XERSGWF6BELZOYQ4L4O` |
| Token issuer | `GDW3QNZSW2QKR5NJCW2GHXHMN6KDO3JRODROJ35JDP255BYLM73YI5YR` |
| Network | Stellar Testnet (`Test SDF Network ; September 2015`) |
| RPC | `https://soroban-testnet.stellar.org` |

`CORD` is a throwaway classic Stellar asset issued solely to exercise the
SEP-41 token transfer path end to end (real accounts need a trustline to
it before they can hold a balance — that's a property of wrapping a
classic asset, not something the escrow contract requires generally). Any
SEP-41-compliant token, including a native Soroban token contract, works
with `concord-escrow`.

### What was verified live on this deployment

Escrow #0 ran the full lifecycle with real funded testnet accounts and
real token transfers:

1. `initialize_escrow` with `client == arbitrator` — rejected with
   `Error::RolesMustBeDistinct` (`Error(Contract, #14)`), confirming the
   role-collision check added after the v1 deployment.
2. `initialize_escrow` with distinct roles — 2 milestones (1000000000,
   2000000000 CORD stroops)
3. `fund_escrow` — client transferred the full 3000000000 into the contract
4. `submit_milestone` / `approve_milestone` on milestone 0 — released
   1000000000 to the provider
5. `raise_dispute` on milestone 1, `resolve_dispute` with `Split(7000)` —
   settled 1400000000 to the provider / 600000000 to the client (70/30,
   exact), which also completed the escrow

Steps 3–5 exercise the reordered (state-before-transfer) code paths from
the reentrancy fix, and every write along the way exercised the new
TTL-extension calls — all produced identical on-chain results to the
pre-hardening v1 deployment's equivalent run.

## Testnet (superseded)

The original Phase 6 deployment, predating the hardening pass. Left
here for history; prefer the current deployment above.

| | |
|---|---|
| Escrow contract | `CCV6TXXCFDFC743XMQBRICV4HFMKRPDDCPUQFKOWVYZ3BNJEGEDQODSO` |
| Wasm hash | `1f5fadd39264da9a782207b266a592da31b550836d2be2293108cec6fb0615e4` |
| Deployed | 2026-09-07 |

That run's full verification notes (including the two indexer bugs
found and fixed against this contract's live event stream) are preserved
in git history for this file.

## Hardening changes (v1 → current)

- State is now updated before calling out to the (caller-supplied)
  token contract in `fund_escrow`/`approve_milestone`/`resolve_dispute`,
  closing a reentrancy path a malicious token could otherwise use to
  get paid twice for the same milestone.
- Every state write now extends the entry's and the contract
  instance's TTL, so a long-idle escrow (or a quiet contract with no
  new escrows) doesn't risk archival.
- `initialize_escrow` now rejects overlapping client/provider/arbitrator
  addresses — previously an arbitrator could also be the client and
  resolve their own dispute in their own favor.

None of this changed the exported function signatures (still the same
10 functions) or the event schema, so `concord-backend`'s indexer needs
no changes to work against the current deployment.

### Reproducing

```bash
# from concord-contracts/
stellar keys generate concord_deployer -n testnet --fund
stellar contract build --out-dir target/wasm-out
stellar contract deploy --wasm target/wasm-out/concord_escrow.wasm \
  --source concord_deployer -n testnet --alias concord_escrow
```
