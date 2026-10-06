// Client helpers shared by the tests and the devnet demo.
import * as anchor from "@anchor-lang/core";
import { createHash } from "crypto";

export const TREE_DEPTH = 20; // must match programs/contract constants::TREE_DEPTH

export const sha256 = (...b: Buffer[]) => createHash("sha256").update(Buffer.concat(b)).digest();

/** commitment = sha256(credential account pubkey), same derivation as the program. */
export const commitmentOf = (credential: anchor.web3.PublicKey) => sha256(credential.toBuffer());

/**
 * Sparse, append-only Merkle tree mirroring the program's (sha256, empty leaf = 32 zero
 * bytes). The chain stores only the root; this is the off-chain copy clients keep.
 */
export class SparseTree {
  private zeros: Buffer[] = [Buffer.alloc(32)];
  private nodes = new Map<string, Buffer>();
  readonly depth: number;
  count = 0;

  constructor(depth: number = TREE_DEPTH) {
    this.depth = depth;
    for (let i = 0; i < depth; i++) this.zeros.push(sha256(this.zeros[i], this.zeros[i]));
  }

  private get(level: number, index: number): Buffer {
    return this.nodes.get(`${level}:${index}`) ?? this.zeros[level];
  }

  root(): Buffer {
    return this.get(this.depth, 0);
  }

  /** Append at the next free index; returns that index. */
  append(leaf: Buffer): number {
    const index = this.count++;
    let node = leaf;
    let idx = index;
    this.nodes.set(`0:${idx}`, node);
    for (let level = 0; level < this.depth; level++) {
      const sibling = this.get(level, idx ^ 1);
      node = idx & 1 ? sha256(sibling, node) : sha256(node, sibling);
      idx >>= 1;
      this.nodes.set(`${level + 1}:${idx}`, node);
    }
    return index;
  }

  /** Sibling hashes bottom-up for `index` (works for filled and for the next free slot). */
  proof(index: number): number[][] {
    const out: number[][] = [];
    for (let level = 0; level < this.depth; level++) out.push([...this.get(level, (index >> level) ^ 1)]);
    return out;
  }
}

export interface Pdas {
  registry: anchor.web3.PublicKey;
  verifierConfig: (verifierKey: anchor.web3.PublicKey, network: anchor.web3.PublicKey) => anchor.web3.PublicKey;
  enrollment: (commitment: Buffer) => anchor.web3.PublicKey;
  nullifier: (consumer: anchor.web3.PublicKey, commitment: Buffer) => anchor.web3.PublicKey;
}

export function pdas(programId: anchor.web3.PublicKey): Pdas {
  const find = (seeds: (Buffer | Uint8Array)[]) => anchor.web3.PublicKey.findProgramAddressSync(seeds, programId)[0];
  return {
    registry: find([Buffer.from("registry")]),
    verifierConfig: (k, n) => find([Buffer.from("verifier_config"), k.toBuffer(), n.toBuffer()]),
    enrollment: (c) => find([Buffer.from("enrollment"), c]),
    nullifier: (consumer, c) => find([Buffer.from("nullifier"), consumer.toBuffer(), c]),
  };
}

/**
 * Rebuild the off-chain tree from on-chain EnrollmentRecords (a mini indexer).
 * Reads only the byte prefix every layout version shares, [discriminator 8][commitment 32]
 * [leaf_index u32 LE], so records written by older program versions still count.
 */
export async function rebuildTree(program: anchor.Program<any>): Promise<SparseTree> {
  const disc: number[] = (program.idl as any).accounts.find((a: any) => a.name.toLowerCase() === "enrollmentrecord").discriminator;
  const accounts = await program.provider.connection.getProgramAccounts(program.programId, {
    filters: [{ memcmp: { offset: 0, bytes: anchor.utils.bytes.bs58.encode(Buffer.from(disc)) } }],
    dataSlice: { offset: 0, length: 44 },
  });
  const leaves = accounts
    .map((a) => ({ index: Buffer.from(a.account.data).readUInt32LE(40), commitment: Buffer.from(a.account.data).subarray(8, 40) }))
    .sort((x, y) => x.index - y.index);
  const tree = new SparseTree();
  for (const l of leaves) {
    if (l.index !== tree.count) throw new Error("gap in leaf indices; cannot rebuild tree");
    tree.append(Buffer.from(l.commitment));
  }
  return tree;
}
