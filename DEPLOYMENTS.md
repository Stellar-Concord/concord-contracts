# Deployments

## Testnet

Deployed 2026-09-07 for Phase 6 integration testing.

| | |
|---|---|
| Escrow contract | `CCV6TXXCFDFC743XMQBRICV4HFMKRPDDCPUQFKOWVYZ3BNJEGEDQODSO` |
| Wasm hash | `1f5fadd39264da9a782207b266a592da31b550836d2be2293108cec6fb0615e4` |
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

### What was verified live

Escrow #0 on the deployed contract ran the full lifecycle with real
funded testnet accounts and real token transfers:

1. `initialize_escrow` — 2 milestones (1000000000, 3000000000 CORD stroops)
2. `fund_escrow` — client transferred the full 4000000000 into the contract
3. `submit_milestone` / `approve_milestone` on milestone 0 — released to provider
4. `raise_dispute` on milestone 1, `resolve_dispute` with `Split(6000)` —
   settled 1800000000 to the provider / 1200000000 to the client (60/40),
   which also completed the escrow

`concord-backend`'s indexer, pointed at this same contract and RPC,
correctly decoded every event (including the nested `milestones` vec and
the `Resolution` union) and its REST API served state that matched the
on-chain history exactly, including `resolution_provider_bps: 6000`.
Running against a live RPC surfaced two real bugs in the indexer
(`startLedger: 0` is rejected; the per-event cursor field is `id`, not
`pagingToken`) — both fixed and covered by a regression test in
`concord-backend`.

The frontend, pointed at this contract's ID, rendered escrow #0's real
state correctly via the backend.

### Reproducing

```bash
# from concord-contracts/
stellar keys generate concord_deployer -n testnet --fund
stellar contract build --out-dir target/wasm-out
stellar contract deploy --wasm target/wasm-out/concord_escrow.wasm \
  --source concord_deployer -n testnet --alias concord_escrow
```
