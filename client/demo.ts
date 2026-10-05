// End-to-end demo: admin setup -> enroll -> gated dApp action via CPI -> replay blocked.
// Run against devnet:  ANCHOR_PROVIDER_URL=https://api.devnet.solana.com pnpm demo
import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import * as fs from "fs";
import * as path from "path";
import type { Contract } from "../target/types/contract";
import type { MockConsumer } from "../target/types/mock_consumer";
import { commitmentOf, pdas, rebuildTree, sha256 } from "./passport";

const { Keypair, SystemProgram, SYSVAR_INSTRUCTIONS_PUBKEY, Transaction } = anchor.web3;

const idl = (name: string) => JSON.parse(fs.readFileSync(path.join(__dirname, "../target/idl", `${name}.json`), "utf8"));

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const passport = new Program<Contract>(idl("contract"), provider);
  const consumer = new Program<MockConsumer>(idl("mock_consumer"), provider);
  const admin = provider.wallet.publicKey;
  const P = pdas(passport.programId);
  const cluster = provider.connection.rpcEndpoint.includes("devnet") ? "?cluster=devnet" : "";
  const link = (sig: string) => `https://explorer.solana.com/tx/${sig}${cluster}`;
  const step = (s: string) => console.log(`\n== ${s}`);

  // Stand-in for the external verifier program (Reclaim) whose credential accounts we trust.
  const verifier = Keypair.fromSeed(sha256(Buffer.from("passport-demo-verifier"))).publicKey;

  step("1. Admin setup (idempotent)");
  if (!(await provider.connection.getAccountInfo(P.registry))) {
    const sig = await passport.methods.initializeRegistry().accountsPartial({ registry: P.registry }).rpc();
    console.log("registry initialized", link(sig));
  } else console.log("registry already initialized");
  const sig = await passport.methods
    .registerVerifier(verifier, true)
    .accountsPartial({ registry: P.registry, verifierConfig: P.verifierConfig(verifier) })
    .rpc();
  console.log("verifier registered + active", link(sig));

  step("2. A new human obtains a credential and enrolls");
  const wallet = Keypair.generate();
  const credential = Keypair.generate();
  await provider.sendAndConfirm(
    new Transaction().add(
      SystemProgram.transfer({ fromPubkey: admin, toPubkey: wallet.publicKey, lamports: 20_000_000 }),
      SystemProgram.createAccount({
        fromPubkey: admin,
        newAccountPubkey: credential.publicKey,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        space: 8,
        programId: verifier, // account owned by the trusted verifier program
      })
    ),
    [credential]
  );
  const commitment = commitmentOf(credential.publicKey);
  const tree = await rebuildTree(passport as Program<any>); // off-chain tree rebuilt from chain state
  const next = tree.count;
  const enrollSig = await passport.methods
    .enrollIdentity([...commitment], tree.proof(next))
    .accountsPartial({
      wallet: wallet.publicKey,
      verifierConfig: P.verifierConfig(verifier),
      credentialAccount: credential.publicKey,
      registry: P.registry,
      enrollment: P.enrollment(commitment),
    })
    .signers([wallet])
    .rpc();
  tree.append(commitment);
  console.log(`enrolled at leaf ${next}`, link(enrollSig));
  const reg = await passport.account.registryConfig.fetch(P.registry);
  console.log("on-chain root matches local tree:", Buffer.from(reg.merkleRoot).equals(tree.root()));

  step("3. A dApp (mock_consumer) gates an action behind verify_personhood via CPI");
  const gated = async () => {
    const ix = await consumer.methods
      .gatedAction([...commitment], tree.proof(next))
      .accountsPartial({
        wallet: wallet.publicKey,
        registry: P.registry,
        enrollment: P.enrollment(commitment),
        nullifier: P.nullifier(consumer.programId, commitment),
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        passportProgram: passport.programId,
      })
      .instruction();
    ix.keys.find((k) => k.pubkey.equals(wallet.publicKey))!.isSigner = true;
    return provider.sendAndConfirm(new Transaction().add(ix), [wallet]);
  };
  console.log("verified, dApp action executed", link(await gated()));

  step("4. Replay with the same identity is rejected");
  try {
    await gated();
    console.log("UNEXPECTED: replay succeeded");
    process.exit(1);
  } catch (e: any) {
    const logs: string = [e.message, ...(e.logs ?? [])].join("\n");
    console.log(logs.includes("NullifierAlreadyUsed") ? "rejected: NullifierAlreadyUsed" : `rejected: ${e.message}`);
  }
  console.log("\nDone.");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
