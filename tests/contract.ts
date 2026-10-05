import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { expect } from "chai";
import { Contract } from "../target/types/contract";
import { MockConsumer } from "../target/types/mock_consumer";
import { SparseTree, commitmentOf, pdas, rebuildTree } from "../client/passport";

const { Keypair, SystemProgram, SYSVAR_INSTRUCTIONS_PUBKEY, Transaction } = anchor.web3;
type PK = anchor.web3.PublicKey;
type User = { wallet: anchor.web3.Keypair; credential: PK; commitment: Buffer };

describe("contract (passport)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const passport = anchor.workspace.contract as Program<Contract>;
  const consumer = anchor.workspace.mockConsumer as Program<MockConsumer>;
  const admin = provider.wallet.publicKey;
  const P = pdas(passport.programId);

  const verifierProgram = Keypair.generate().publicKey; // stand-in for the Reclaim verifier
  const tree = new SparseTree();

  async function newUser(owner: PK = verifierProgram): Promise<User> {
    const wallet = Keypair.generate();
    const credential = Keypair.generate();
    const tx = new Transaction().add(
      SystemProgram.transfer({ fromPubkey: admin, toPubkey: wallet.publicKey, lamports: 50_000_000 }),
      SystemProgram.createAccount({
        fromPubkey: admin,
        newAccountPubkey: credential.publicKey,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        space: 8,
        programId: owner,
      })
    );
    await provider.sendAndConfirm(tx, [credential]);
    return { wallet, credential: credential.publicKey, commitment: commitmentOf(credential.publicKey) };
  }

  const enroll = (u: User, opts: { proof?: number[][]; verifier?: PK } = {}) =>
    passport.methods
      .enrollIdentity([...u.commitment], opts.proof ?? tree.proof(tree.count))
      .accountsPartial({
        wallet: u.wallet.publicKey,
        verifierConfig: P.verifierConfig(opts.verifier ?? verifierProgram),
        credentialAccount: u.credential,
        registry: P.registry,
        enrollment: P.enrollment(u.commitment),
      })
      .signers([u.wallet]);

  async function gatedIx(u: User, leafIndex: number, proof?: number[][]) {
    const ix = await consumer.methods
      .gatedAction([...u.commitment], proof ?? tree.proof(leafIndex))
      .accountsPartial({
        wallet: u.wallet.publicKey,
        registry: P.registry,
        enrollment: P.enrollment(u.commitment),
        nullifier: P.nullifier(consumer.programId, u.commitment),
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        passportProgram: passport.programId,
      })
      .instruction();
    return ix;
  }

  /** Calls verify_personhood through the mock consumer (a real CPI). */
  async function gatedAction(u: User, leafIndex: number, opts: { sign?: boolean; proof?: number[][] } = {}) {
    const ix = await gatedIx(u, leafIndex, opts.proof);
    // The consumer takes `wallet` as an unchecked account, so set the signer bit by hand.
    ix.keys.find((k) => k.pubkey.equals(u.wallet.publicKey))!.isSigner = opts.sign !== false;
    return provider.sendAndConfirm(new Transaction().add(ix), opts.sign === false ? [] : [u.wallet]);
  }

  async function expectFail(p: Promise<unknown>, needle: string) {
    try {
      await p;
    } catch (e: any) {
      const text = [e?.message, e?.error?.errorMessage, ...(e?.logs ?? []), ...(e?.transactionLogs ?? [])].join("\n");
      if (!text.includes(needle)) console.log(text);
      expect(text, `expected error containing "${needle}"`).to.include(needle);
      return;
    }
    expect.fail(`expected failure containing "${needle}"`);
  }

  let alice: User, bob: User, carol: User;

  before(async () => {
    alice = await newUser();
    bob = await newUser();
    carol = await newUser();
  });

  describe("admin setup", () => {
    it("initialize_registry stores the empty-tree root", async () => {
      await passport.methods.initializeRegistry().accountsPartial({ registry: P.registry }).rpc();
      const reg = await passport.account.registryConfig.fetch(P.registry);
      expect(Buffer.from(reg.merkleRoot).equals(tree.root())).to.equal(true);
      expect(reg.admin.equals(admin)).to.equal(true);
      expect(reg.isFrozen).to.equal(false);
      expect(reg.treeDepth).to.equal(20);
    });

    it("register_verifier: only the admin may register", async () => {
      const mallory = Keypair.generate();
      await provider.sendAndConfirm(
        new Transaction().add(SystemProgram.transfer({ fromPubkey: admin, toPubkey: mallory.publicKey, lamports: 20_000_000 })),
        []
      );
      await expectFail(
        passport.methods
          .registerVerifier(verifierProgram, true)
          .accountsPartial({ admin: mallory.publicKey, registry: P.registry, verifierConfig: P.verifierConfig(verifierProgram) })
          .signers([mallory])
          .rpc(),
        "Unauthorized"
      );
      await passport.methods
        .registerVerifier(verifierProgram, true)
        .accountsPartial({ registry: P.registry, verifierConfig: P.verifierConfig(verifierProgram) })
        .rpc();
      const v = await passport.account.verifierConfig.fetch(P.verifierConfig(verifierProgram));
      expect(v.isActive).to.equal(true);
    });
  });

  describe("enroll_identity", () => {
    it("happy path: enrolls and stores the recomputed root", async () => {
      await enroll(alice).rpc();
      tree.append(alice.commitment);
      const reg = await passport.account.registryConfig.fetch(P.registry);
      expect(Buffer.from(reg.merkleRoot).equals(tree.root())).to.equal(true);
      expect(reg.nextLeafIndex).to.equal(1);
      const rec = await passport.account.enrollmentRecord.fetch(P.enrollment(alice.commitment));
      expect(rec.leafIndex).to.equal(0);
      expect(rec.wallet.equals(alice.wallet.publicKey)).to.equal(true);
      expect(rec.credential.equals(alice.credential)).to.equal(true);
      expect(rec.enrolledAt.toNumber()).to.be.greaterThan(0);
    });

    it("rejects a duplicate enrollment with AlreadyEnrolled", async () => {
      await expectFail(enroll(alice).rpc(), "AlreadyEnrolled");
    });

    it("rejects a credential not owned by the verifier program", async () => {
      await expectFail(enroll(await newUser(SystemProgram.programId)).rpc(), "InvalidCredentialOwner");
    });

    it("rejects a forged Merkle proof", async () => {
      const bad = tree.proof(1);
      bad[0][0] ^= 0xff;
      await expectFail(enroll(bob, { proof: bad }).rpc(), "InvalidMerkleProof");
    });

    it("rejects a commitment that is not sha256(credential)", async () => {
      const wrong = { ...bob, commitment: Buffer.alloc(32, 7) };
      await expectFail(enroll(wrong as User).rpc(), "InvalidCommitment");
    });

    it("rejects an inactive verifier, accepts it again once reactivated", async () => {
      const setActive = (a: boolean) =>
        passport.methods
          .registerVerifier(verifierProgram, a)
          .accountsPartial({ registry: P.registry, verifierConfig: P.verifierConfig(verifierProgram) })
          .rpc();
      await setActive(false);
      await expectFail(enroll(bob).rpc(), "VerifierNotActive");
      await setActive(true);
    });

    it("enrolls more identities at consecutive leaves", async () => {
      for (const u of [bob, carol]) {
        await enroll(u).rpc();
        tree.append(u.commitment);
      }
      const reg = await passport.account.registryConfig.fetch(P.registry);
      expect(Buffer.from(reg.merkleRoot).equals(tree.root())).to.equal(true);
      expect(reg.nextLeafIndex).to.equal(3);
    });

    it("the off-chain tree can be rebuilt from chain state", async () => {
      const rebuilt = await rebuildTree(passport as Program<any>);
      expect(rebuilt.root().equals(tree.root())).to.equal(true);
    });
  });

  describe("verify_personhood (via CPI)", () => {
    it("happy path: creates the nullifier", async () => {
      await gatedAction(alice, 0);
      const n = await passport.account.nullifierRecord.fetch(P.nullifier(consumer.programId, alice.commitment));
      expect(n.createdAt.toNumber()).to.be.greaterThan(0);
    });

    it("rejects a replay with NullifierAlreadyUsed", async () => {
      await expectFail(gatedAction(alice, 0), "NullifierAlreadyUsed");
    });

    it("rejects when the wallet did not sign", async () => {
      await expectFail(gatedAction(bob, 1, { sign: false }), "signer privilege escalated");
    });

    it("passport's own Signer check fires without the wallet signature", async () => {
      const ix = await passport.methods
        .verifyPersonhood([...bob.commitment], consumer.programId, tree.proof(1))
        .accountsPartial({
          wallet: bob.wallet.publicKey,
          registry: P.registry,
          enrollment: P.enrollment(bob.commitment),
          nullifier: P.nullifier(consumer.programId, bob.commitment),
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .instruction();
      ix.keys.find((k) => k.pubkey.equals(bob.wallet.publicKey))!.isSigner = false;
      await expectFail(provider.sendAndConfirm(new Transaction().add(ix), []), "AccountNotSigner");
    });

    it("rejects a signer presenting someone else's commitment", async () => {
      const mallory = await newUser();
      await expectFail(gatedAction({ ...bob, wallet: mallory.wallet }, 1), "WalletMismatch");
    });

    it("rejects a direct (non-CPI) call", async () => {
      await expectFail(
        passport.methods
          .verifyPersonhood([...bob.commitment], consumer.programId, tree.proof(1))
          .accountsPartial({
            wallet: bob.wallet.publicKey,
            registry: P.registry,
            enrollment: P.enrollment(bob.commitment),
            nullifier: P.nullifier(consumer.programId, bob.commitment),
            instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          })
          .signers([bob.wallet])
          .rpc(),
        "NotCalledViaCpi"
      );
    });

    it("rejects an invalid Merkle proof", async () => {
      const bad = tree.proof(1);
      bad[1][0] ^= 0xff;
      await expectFail(gatedAction(bob, 1, { proof: bad }), "InvalidMerkleProof");
    });
  });

  describe("freeze_identity / registry freeze", () => {
    const freeze = (u: User, frozen: boolean, signer?: anchor.web3.Keypair) =>
      passport.methods
        .freezeIdentity([...u.commitment], frozen)
        .accountsPartial({
          admin: signer?.publicKey ?? admin,
          registry: P.registry,
          enrollment: P.enrollment(u.commitment),
        })
        .signers(signer ? [signer] : []);

    it("only the admin can freeze an identity", async () => {
      await expectFail(freeze(carol, true, carol.wallet).rpc(), "Unauthorized");
    });

    it("a frozen identity fails verification; unfreezing restores it", async () => {
      await freeze(carol, true).rpc();
      await expectFail(gatedAction(carol, 2), "IdentityFrozen");
      await freeze(carol, false).rpc();
      await gatedAction(carol, 2);
    });

    it("registry freeze blocks enroll and verify; unfreezing restores them", async () => {
      const setFrozen = (f: boolean) =>
        passport.methods.setRegistryFrozen(f).accountsPartial({ registry: P.registry }).rpc();
      const dave = await newUser();
      await setFrozen(true);
      await expectFail(enroll(dave).rpc(), "RegistryFrozen");
      await expectFail(gatedAction(bob, 1), "RegistryFrozen");
      await setFrozen(false);
      await enroll(dave).rpc();
      tree.append(dave.commitment);
      await gatedAction(bob, 1);
    });
  });
});
