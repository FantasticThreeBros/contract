# Passport: privacy-preserving proof-of-personhood registry (Solana / Anchor)

One human, one identity, without revealing who. A person enrolls a commitment derived from their
verifier credential; dApps later ask for proof of personhood via CPI and get a per-dApp nullifier
(replay guard) instead of a global identifier.



Signature: bXnqqn2bB1Mwz3FSZSCrC8SfXPj6MCyqLCfhJhXYitpBAiN4RuJgQMcWFW88v1PYHs9rzzZjfaC87UQaCDWMXAK
Program ID: 6yH5EwvxmJbKoVgYPni6NMaLwNChgen23CkF9omvYbDE

Writing metadata account...
 ├─ metadata: 6WN6HMGgzEgeb2tuBBbwc1M2MLgTMjD829EsJvMpQoNc
 ├─ program: 6yH5EwvxmJbKoVgYPni6NMaLwNChgen23CkF9omvYbDE
 └─ seed: idl

[Success] Operation executed successfully
IDL initialized.
✓ Idl metadata created/updated
Deploying program "mock_consumer"...
Program path: /Users/dhruvilpatel/Developer/capstone/contract/target/deploy/mock_consumer.so...
Program already exists, upgrading...
Sending upgrade transaction...
Signature: 61bqimKRrRdyXuVYBQoZeZcZAmFqCyuigCrj4KBAwVNJxn2FiXuU4AasDYbx7tdYP3MxpoWd4uQwQpHwG3rz1u3J
Program ID: 7ZBun553WhQd3iruZoxvDPj46uDjY4bsZZGqa2Aw7i68

Writing metadata account...
 ├─ metadata: 6k51Wr2oQX5ULySe7hZTYkCjzagRxXH22Lrtiz9MP87c
 ├─ program: 7ZBun553WhQd3iruZoxvDPj46uDjY4bsZZGqa2Aw7i68
 └─ seed: idl

[Success] Operation executed successfully
IDL initialized.
✓ Idl metadata created/updated
Deploy success








## Instructions
| Instruction | Who | What |
|---|---|---|
| `initialize_registry` | admin | creates the registry with the empty-tree Merkle root |
| `register_verifier(key, is_active)` | admin | trust (or deactivate) a verifier program |
| `enroll_identity(commitment, proof)` | wallet | reads the verifier's credential account (owner-checked, no CPI), `commitment = sha256(credential key)`, dedups via the `["enrollment", commitment]` PDA, appends the leaf, stores the new root |
| `verify_personhood(commitment, consumer, proof)` | wallet, **via CPI** | wallet must sign; checks Merkle proof; creates `["nullifier", consumer, commitment]` or fails with `NullifierAlreadyUsed` |
| `freeze_identity(commitment, frozen)` | admin | freeze/unfreeze one enrollment |
| `set_registry_frozen(frozen)` | admin | global kill switch for enroll + verify |

## Layout
- `programs/contract`: the registry program
- `programs/mock-consumer`: demo dApp that gates an action behind `verify_personhood` via CPI
- `client/passport.ts`: sparse Merkle tree + PDA helpers (rebuilds the tree from chain state)
- `client/demo.ts`: end-to-end demo; `tests/contract.ts`: 20 mocha tests

```bash
anchor build && anchor test --skip-build      # local validator
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com ANCHOR_WALLET=~/.config/solana/id.json \
  pnpm exec ts-node -P tsconfig.json client/demo.ts   # full loop on devnet
```

## POC simplifications / known limitations
- **Merkle tree is off-chain**; only the root + next leaf index are on-chain. Concurrent enrolls
  invalidate each other's proofs (client re-fetches and retries). Production: SPL account-compression.
- **Credential layout is stubbed** (`credential.rs`, `CredentialReader`). Until the Reclaim layout is
  wired in (https://docs.reclaimprotocol.org/solana/quickstart) the credential is not bound to the
  enrolling wallet, so someone could enroll with another person's credential account.
- `verify_personhood` identifies the *top-level* program as the consumer; nested CPI chains
  (dApp -> helper -> passport) are attributed to the top-level program.
