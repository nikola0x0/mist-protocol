/**
 * Privacy Relayer API Route
 *
 * This endpoint receives encrypted swap intents from users and submits them
 * to the Sui blockchain from the relayer's wallet, breaking the link between
 * the user's wallet and their swap intent.
 *
 * Flow:
 * 1. User creates encrypted intent (SEAL) with wallet signature
 * 2. User POSTs to this endpoint (no wallet connection needed)
 * 3. Relayer submits create_swap_intent tx from RELAYER wallet
 * 4. TEE polls chain, verifies signature, executes swap
 *
 * Privacy: Observer sees "Relayer submitted intent" not "User submitted intent"
 */

import { NextRequest, NextResponse } from "next/server";
import { SuiClient } from "@mysten/sui/client";
import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";
import { Transaction } from "@mysten/sui/transactions";
import { fromHex } from "@mysten/sui/utils";

// ============ CONFIG ============

const PACKAGE_ID = process.env.NEXT_PUBLIC_PACKAGE_ID || "";
const NETWORK = process.env.NEXT_PUBLIC_NETWORK || "testnet";
const RELAYER_PRIVATE_KEY = process.env.RELAYER_PRIVATE_KEY || "";

const RPC_URL =
  NETWORK === "mainnet"
    ? "https://fullnode.mainnet.sui.io"
    : "https://fullnode.testnet.sui.io";

// ============ TYPES ============

interface SubmitIntentRequest {
  /** Base64 SEAL encrypted intent details (includes signature) */
  encryptedDetails: string;
  /** Input token type (e.g., "SUI") */
  tokenIn: string;
  /** Output token type (e.g., "SUI") */
  tokenOut: string;
  /** Deadline in milliseconds (optional, defaults to +1 hour) */
  deadline?: number;
}

interface SubmitIntentResponse {
  success: boolean;
  txDigest?: string;
  error?: string;
}

// ============ HELPERS ============

/**
 * Load relayer keypair from environment variable
 * Supports both hex format and Bech32 (suiprivkey1...) format
 */
function loadRelayerKeypair(): Ed25519Keypair {
  if (!RELAYER_PRIVATE_KEY) {
    throw new Error("RELAYER_PRIVATE_KEY not configured");
  }

  // Check if it's Bech32 format (suiprivkey1...)
  if (RELAYER_PRIVATE_KEY.startsWith("suiprivkey")) {
    // Decode Bech32
    const { bech32 } = require("bech32");
    const decoded = bech32.decode(RELAYER_PRIVATE_KEY);
    const bytes = bech32.fromWords(decoded.words);
    // First byte is scheme (0x00 for ed25519), rest is 32-byte key
    const keyBytes = new Uint8Array(bytes.slice(1));
    return Ed25519Keypair.fromSecretKey(keyBytes);
  }

  // Assume hex format (with or without 0x prefix)
  const hexKey = RELAYER_PRIVATE_KEY.startsWith("0x")
    ? RELAYER_PRIVATE_KEY.slice(2)
    : RELAYER_PRIVATE_KEY;
  const keyBytes = fromHex(`0x${hexKey}`);
  return Ed25519Keypair.fromSecretKey(keyBytes);
}

// ============ ROUTE HANDLER ============

export async function POST(request: NextRequest): Promise<NextResponse<SubmitIntentResponse>> {
  try {
    // Parse request body
    const body: SubmitIntentRequest = await request.json();

    // Validate required fields
    if (!body.encryptedDetails) {
      return NextResponse.json(
        { success: false, error: "encryptedDetails is required" },
        { status: 400 }
      );
    }

    if (!body.tokenIn || !body.tokenOut) {
      return NextResponse.json(
        { success: false, error: "tokenIn and tokenOut are required" },
        { status: 400 }
      );
    }

    // Check relayer configuration
    if (!RELAYER_PRIVATE_KEY) {
      return NextResponse.json(
        { success: false, error: "Relayer not configured" },
        { status: 500 }
      );
    }

    if (!PACKAGE_ID) {
      return NextResponse.json(
        { success: false, error: "Package ID not configured" },
        { status: 500 }
      );
    }

    // Load relayer keypair
    const relayerKeypair = loadRelayerKeypair();
    const relayerAddress = relayerKeypair.toSuiAddress();

    console.log(`[Relayer] Received intent submission request`);
    console.log(`[Relayer] Submitting from: ${relayerAddress}`);

    // Initialize Sui client
    const suiClient = new SuiClient({ url: RPC_URL });

    // Default deadline: 1 hour from now
    const deadline = body.deadline || Date.now() + 60 * 60 * 1000;

    // Build transaction
    const tx = new Transaction();

    tx.moveCall({
      target: `${PACKAGE_ID}::mist_protocol::create_swap_intent`,
      arguments: [
        tx.pure.vector("u8", Array.from(new TextEncoder().encode(body.encryptedDetails))),
        tx.pure.vector("u8", Array.from(new TextEncoder().encode(body.tokenIn))),
        tx.pure.vector("u8", Array.from(new TextEncoder().encode(body.tokenOut))),
        tx.pure.u64(deadline),
      ],
    });

    // Sign and execute transaction
    const result = await suiClient.signAndExecuteTransaction({
      signer: relayerKeypair,
      transaction: tx,
    });

    console.log(`[Relayer] Intent submitted: ${result.digest}`);

    return NextResponse.json({
      success: true,
      txDigest: result.digest,
    });
  } catch (error) {
    console.error("[Relayer] Error:", error);

    const errorMessage =
      error instanceof Error ? error.message : "Unknown error";

    return NextResponse.json(
      { success: false, error: errorMessage },
      { status: 500 }
    );
  }
}

// Health check
export async function GET(): Promise<NextResponse> {
  const configured = !!RELAYER_PRIVATE_KEY && !!PACKAGE_ID;

  let relayerAddress = "";
  if (configured) {
    try {
      const keypair = loadRelayerKeypair();
      relayerAddress = keypair.toSuiAddress();
    } catch {
      // Ignore
    }
  }

  return NextResponse.json({
    status: configured ? "ready" : "not_configured",
    relayerAddress: relayerAddress || undefined,
    network: NETWORK,
  });
}
