// End-to-end demo: admin setup -> enroll -> gated dApp action via CPI -> replay blocked.
// Default: the mock program issues a pass in Civic's exact layout (no real Civic pass needed).
// REAL_CIVIC=1: use the admin wallet's REAL Civic pass instead (set CIVIC_NETWORK to its network).
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

  const GATEWAY = new anchor.web3.PublicKey("gatem74V238djXdzWnJf94Wo1DcnuGkfijbf3AuBhfs"); // Civic
  const UNIQUENESS = "uniqobk8oGh4XBLMqM68K8M2zNu3CdYX7q5go7whQiv";
  const real = process.env.REAL_CIVIC === "1";
  const network = new anchor.web3.PublicKey(process.env.CIVIC_NETWORK ?? UNIQUENESS);
  // Who owns credential accounts: Civic's gateway program, or our mock issuer.
  const verifier = real ? GATEWAY : consumer.programId;

  step("1. Admin setup (idempotent)");
  if (!(await provider.connection.getAccountInfo(P.registry))) {
    const sig = await passport.methods.initializeRegistry().accountsPartial({ registry: P.registry }).rpc();
    console.log("registry initialized", link(sig));
  } else console.log("registry already initialized");
  const sig = await passport.methods
    .registerVerifier(verifier, network, true)
    .accountsPartial({ registry: P.registry, verifierConfig: P.verifierConfig(verifier, network) })
    .rpc();
  console.log(`verifier registered (${real ? "Civic gateway" : "mock issuer"}), network pinned: ${network.toBase58()}`, link(sig));

  step("2. A human holds a pass and enrolls");
  let wallet: anchor.web3.Keypair;
  let credential: anchor.web3.PublicKey;
  if (real) {
    // Civic's pass PDA: [wallet, "gateway", 8 zero bytes (no extra seed), network]
    wallet = (provider.wallet as anchor.Wallet & { payer: anchor.web3.Keypair }).payer;
    credential = anchor.web3.PublicKey.findProgramAddressSync(
      [wallet.publicKey.toBuffer(), Buffer.from("gateway"), Buffer.alloc(8), network.toBuffer()],
      GATEWAY
    )[0];
    if (!(await provider.connection.getAccountInfo(credential))) {
      console.error(`No Civic pass for ${wallet.publicKey} on network ${network}. Get one via Civic, then retry.`);
      process.exit(1);
    }
    console.log("using real Civic pass", credential.toBase58());
  } else {
    wallet = Keypair.generate();
    credential = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("demo_pass"), wallet.publicKey.toBuffer(), network.toBuffer()],
      consumer.programId
    )[0];
    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.transfer({ fromPubkey: admin, toPubkey: wallet.publicKey, lamports: 20_000_000 }),
        await consumer.methods
          .issueDemoPass(wallet.publicKey, network, 0, new anchor.BN(4_102_444_800))
          .accountsPartial({ payer: admin, pass: credential })
          .instruction()
      ),
      []
    );
    console.log("issued demo pass", credential.toBase58());
  }
  const commitment = commitmentOf(credential);
  const tree = await rebuildTree(passport as Program<any>); // off-chain tree rebuilt from chain state
  const next = tree.count;
  const enrollSig = await passport.methods
    .enrollIdentity([...commitment], tree.proof(next))
    .accountsPartial({
      wallet: wallet.publicKey,
      verifierConfig: P.verifierConfig(verifier, network),
      credentialAccount: credential,
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
