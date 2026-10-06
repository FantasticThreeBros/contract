# Passport: privacy-preserving proof-of-personhood registry (Solana / Anchor)

One human, one identity, without revealing who. A person enrolls a commitment derived from their
verifier credential; dApps later ask for proof of personhood via CPI and get a per-dApp nullifier
(replay guard) instead of a global identifier.

## Devnet deployment
- `contract` (registry): `6yH5EwvxmJbKoVgYPni6NMaLwNChgen23CkF9omvYbDE`
- `mock_consumer` (demo dApp + demo pass issuer): `7ZBun553WhQd3iruZoxvDPj46uDjY4bsZZGqa2Aw7i68`

## Instructions
| Instruction | Who | What |
|---|---|---|
| `initialize_registry` | admin | creates the registry with the empty-tree Merkle root |
| `register_verifier(key, network, is_active)` | admin | trust a verifier program **and pin one issuing network** (or deactivate it) |
| `enroll_identity(commitment, proof)` | wallet | reads the Civic Pass account (no CPI) and checks: owned by the verifier program, issued by the **pinned network**, active, unexpired, and issued to the **signing wallet**; then `commitment = sha256(credential key)`, dedups via the `["enrollment", commitment]` PDA, appends the leaf, stores the new root |
| `verify_personhood(commitment, consumer, proof)` | wallet, **via CPI** | wallet must sign; checks Merkle proof; creates `["nullifier", consumer, commitment]` or fails with `NullifierAlreadyUsed` |
| `freeze_identity(commitment, frozen)` | admin | freeze/unfreeze one enrollment |
| `set_registry_frozen(frozen)` | admin | global kill switch for enroll + verify |

## Layout
- `programs/contract`: the registry program
- `programs/mock-consumer`: demo dApp that gates an action behind `verify_personhood` via CPI
- `client/passport.ts`: sparse Merkle tree + PDA helpers (rebuilds the tree from chain state)
- `client/demo.ts`: end-to-end demo; `tests/contract.ts`: 24 mocha tests

```bash
anchor build && anchor test --skip-build      # local validator
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com ANCHOR_WALLET=~/.config/solana/id.json \
  pnpm exec ts-node -P tsconfig.json client/demo.ts   # full loop on devnet
```

## Credential: Civic Pass
`enroll_identity` reads a Civic Pass ("gateway token"), an account owned by Civic's gateway program
(`gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs`, same id on devnet and mainnet). The parser
(`credential.rs`) mirrors Civic's borsh layout and is unit-tested against **real devnet accounts**.
Register the gateway program as the verifier and Civic's **Uniqueness Pass** network
(`uniqobk8oGh4XBLMqM68K8M2zNu3CdYX7q5go7whQiv`, taken from Civic's repo; confirm in Civic's dashboard)
to get one-human-one-pass. **Pinning the network matters**: anyone can create their own gatekeeper
network under the same program, so an owner check alone would accept a self-minted pass.

Reclaim Protocol was evaluated first, but its Solana program is stateless (it verifies a proof
inside the transaction and leaves no credential account), so it doesn't fit an account-read design.

For tests and the demo, `mock-consumer` has `issue_demo_pass`, which writes an account in Civic's
exact layout. `REAL_CIVIC=1` makes the demo use the admin wallet's real pass instead.

## POC simplifications / known limitations
- **Pseudonymous, not anonymous.** No zero-knowledge. The wallet-to-commitment link is on-chain and
  the commitment is the same across apps, so apps can link a user; the per-app nullifier only stops
  replay within one app. Civic passes are public and wallet-linked. A ZK membership proof is the
  next step.
- **Uniqueness relies on Civic's Uniqueness Pass** (one user, one wallet); we add one enrollment per pass.
- The pass is checked at enroll time only; later revocation does not remove an enrollment.
- **Merkle tree is off-chain**; only the root + next leaf index are on-chain. Concurrent enrolls
  invalidate each other's proofs (client re-fetches and retries). Production: SPL account-compression.
- `verify_personhood` identifies the *top-level* program as the consumer; nested CPI chains
  (dApp -> helper -> passport) are attributed to the top-level program.
