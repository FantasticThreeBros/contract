# Solana Passport: pitch script and Q&A

## 3-minute script (matches the 13 slides)

**1. Cover (0:00).** We built Solana Passport: one human, one identity, replay-proof, for any Solana app.

**2. Problem (0:15).** Anyone can fake being many people. Airdrops get farmed by one person with a thousand wallets. Votes get skewed. And the usual fix, collecting ID, turns every app into a data honeypot. Apps really need one answer: is this a real human, and have they been here before?

**3. Idea (0:40).** Think of a members-only club. A checker, Civic Pass, verifies you once off-chain and issues an on-chain pass. Our registry is the guest book: you get one entry, ever. Each app is a room that stamps your hand once. We never store who you are, only that you were checked.

**4. Architecture (1:00).** Three actors, six instructions, four accounts. The admin sets up the registry and the trusted verifier. A human enrolls once. A dApp verifies on every gated action, via CPI, in the same transaction.

**5. Stack (1:15).** Solana and Anchor for speed and safety. PDAs, so duplicates can't exist. A Merkle tree, so one 32-byte root represents everyone. SHA-256, CPI, and Civic Pass as the real credential.

**6. Enroll (1:30).** We read the Civic pass directly. The commitment is the hash of the pass address, derived, never chosen by the user, so one person can't enroll twice with different secrets. The enrollment account is seeded by that commitment, so a second attempt fails.

**7. The Civic insight (1:50).** Checking that an account is owned by Civic's program is not enough: anyone can create their own Civic network and mint themselves a pass. So the registry pins the one network it trusts, checks the pass is active and unexpired, and checks it belongs to the signing wallet.

**8. Merkle tree (2:10).** Only the root lives on-chain. The client keeps the tree and sends sibling hashes; we recompute the root and compare. Depth 20 holds over a million identities.

**9. Verify (2:25).** The wallet must sign, so nobody can probe you without consent. The call must arrive by CPI from the real calling program. The proof must match the root. Then we create a nullifier, one per app per person: use it twice and it fails.

**10. Demo (2:40).** [Run the demo or play the backup video.] Set up, enroll, a dApp action gated by CPI, and the replay is rejected.

**11. Honest limits (2:55).** This is pseudonymous today, not anonymous. Zero-knowledge membership proofs are the next step. Thank you, happy to take questions.

> Trim slide 10 (security table) to a glance if you are short on time; it is the backup for Q&A.

## Q&A cheat-sheet

**Why can't someone enroll twice?** The commitment is sha256 of the Civic pass address, and the enrollment PDA is seeded by it. A second enrollment hits an existing account and fails with `AlreadyEnrolled`. Civic's Uniqueness Pass also limits a human to one pass.

**What stops a fake credential?** Two checks: the account must be owned by the trusted verifier program, and the issuing network must equal the one pinned in `VerifierConfig`. The owner check alone is not enough because anyone can create a network.

**What stops enrolling with someone else's pass?** The pass records its owner wallet, and `enroll_identity` requires it to equal the signer.

**What stops replay?** `verify_personhood` creates a nullifier PDA seeded by app id and commitment. A second call finds it and fails with `NullifierAlreadyUsed`.

**Can another program check me without my consent?** No. The wallet must sign `verify_personhood`, and the signature requirement is enforced by the program.

**How do you know which app is calling?** From the transaction's instruction list (instructions sysvar), not from an argument. Limitation: with a nested CPI chain it reports the top-level program.

**Is it private?** Pseudonymous, not anonymous. The wallet-to-commitment link is on-chain and the commitment is the same across apps, so apps could link a user. The nullifier only stops replay inside one app. A zero-knowledge membership proof is the planned fix.

**Why is the Merkle tree off-chain?** Cost. Only the root is on-chain. Trade-off: two simultaneous enrollments invalidate each other's proofs and the client retries. Production would use SPL account compression.

**Why Civic and not Reclaim?** Reclaim's Solana program is stateless: it verifies a proof inside the transaction and leaves no credential account. Our design reads an account, which Civic provides. Reclaim would need a CPI-based redesign.

**Is the demo using a real Civic pass?** The demo uses a mock issuer that writes an account in Civic's exact layout. The parser is unit-tested against real Civic pass accounts from devnet. A real-pass mode exists (`REAL_CIVIC=1`) but has not been run end to end.

**What if a pass is revoked after enrollment?** We check at enroll time only. The admin can freeze an identity or the whole registry. Re-checking on verify is a possible extension.

**What would you do next?** Zero-knowledge membership proofs, a concurrent Merkle tree, and a real Civic Uniqueness Pass run end to end.

**How many tests?** 25 integration tests plus 5 unit tests, including attacker cases: untrusted network, revoked or expired pass, someone else's pass, replay, missing signature, direct non-CPI call, forged proof.
