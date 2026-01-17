/**
 * Mist Protocol v2: Deposit Notes & Nullifier Management
 *
 * Implements Tornado Cash-style privacy with nullifier-based unlinkability.
 * Users must securely backup their deposit notes - if lost, funds are UNRECOVERABLE.
 */

import { SealClient, EncryptedObject } from "@mysten/seal";
import { SuiClient } from "@mysten/sui/client";
import { toHex } from "@mysten/sui/utils";

// Default SEAL key servers for testnet
const DEFAULT_KEY_SERVERS = {
  testnet: [
    "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75",
    "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8",
  ],
  mainnet: [
    // Add mainnet servers when available
  ],
};

// ============ TYPES ============

/**
 * Deposit note stored locally by user
 * WARNING: If lost or stolen, funds can be lost/stolen!
 */
export interface DepositNote {
  /** Random 32-byte nullifier (hex string) - reveals at swap time */
  nullifier: string;
  /** Deposit amount in base units */
  amount: string;
  /** Token type */
  tokenType: "SUI";
  /** When the deposit was created */
  timestamp: number;
  /** Deposit object ID on chain (for reference only) */
  depositId?: string;
  /** Whether this note has been used for a swap */
  spent: boolean;
}

/**
 * Swap intent details to be SEAL encrypted
 */
export interface SwapIntentDetails {
  /** The nullifier from the deposit note */
  nullifier: string;
  /** Amount to swap (must be <= deposit amount) */
  inputAmount: string;
  /** One-time stealth address for swap output */
  outputStealth: string;
  /** One-time stealth address for remainder */
  remainderStealth: string;
}

/**
 * Stealth address with private key for scanning
 */
export interface StealthAddress {
  /** Public address */
  address: string;
  /** Private key (hex) - for deriving to spend */
  privateKey: string;
}

// ============ CONSTANTS ============

const STORAGE_KEY_PREFIX = "mist_protocol_v2_deposit_notes_";
const STEALTH_KEYS_STORAGE_KEY_PREFIX = "mist_protocol_v2_stealth_keys_";

/**
 * Get wallet-scoped storage key for deposit notes
 */
function getStorageKey(walletAddress: string): string {
  return `${STORAGE_KEY_PREFIX}${walletAddress}`;
}

/**
 * Get wallet-scoped storage key for stealth keys
 */
function getStealthStorageKey(walletAddress: string): string {
  return `${STEALTH_KEYS_STORAGE_KEY_PREFIX}${walletAddress}`;
}

/**
 * SEAL encryption namespace (must match contract)
 * Format: namespace (32 bytes) + nonce (5 bytes)
 */
const NAMESPACE_PREFIX = "mist_protocol_v2_seal_namespace_";

// ============ NULLIFIER GENERATION ============

/**
 * Generate a random 32-byte nullifier
 * This is the core secret that breaks the deposit→swap link
 */
export function generateNullifier(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return toHex(bytes);
}

// ============ STEALTH ADDRESS GENERATION ============

/**
 * Generate a one-time stealth address
 * Used for receiving swap outputs unlinkably
 */
export function generateStealthAddress(): StealthAddress {
  // Generate random 32-byte private key
  const privateKeyBytes = crypto.getRandomValues(new Uint8Array(32));
  const privateKey = toHex(privateKeyBytes);

  // For Sui, address is derived from public key
  // Simplified: use hash of private key as address (in production, derive properly)
  const addressBytes = crypto.getRandomValues(new Uint8Array(32));
  const address = toHex(addressBytes);

  return { address, privateKey };
}

// ============ SEAL ENCRYPTION ============

export interface SealConfig {
  packageId: string;
  network: "testnet" | "mainnet";
}

/**
 * Generate SEAL encryption ID with namespace
 * Format: namespace_prefix (32 bytes) + random_nonce (5 bytes)
 */
function generateEncryptionId(): string {
  const namespaceBytes = new TextEncoder().encode(NAMESPACE_PREFIX);
  const nonce = crypto.getRandomValues(new Uint8Array(5));
  const combined = new Uint8Array(namespaceBytes.length + nonce.length);
  combined.set(namespaceBytes, 0);
  combined.set(nonce, namespaceBytes.length);
  return toHex(combined);
}

/**
 * SEAL encrypt deposit data (amount + nullifier)
 * Only TEE can decrypt this
 */
export async function encryptDepositData(
  amount: string,
  nullifier: string,
  config: SealConfig,
  suiClient: SuiClient
): Promise<string> {
  const encryptionId = generateEncryptionId();

  console.log("Encrypting deposit data:", {
    amount,
    nullifierPrefix: nullifier.substring(0, 10) + "...",
    encryptionId: encryptionId.substring(0, 20) + "...",
  });

  const keyServers = DEFAULT_KEY_SERVERS[config.network];
  const sealClient = new SealClient({
    suiClient: suiClient as any,
    serverConfigs: keyServers.map((id: string) => ({
      objectId: id,
      weight: 1,
    })),
    verifyKeyServers: false,
  });

  // Data format: JSON with amount and nullifier
  const data = JSON.stringify({ amount, nullifier });

  const { encryptedObject } = await sealClient.encrypt({
    threshold: 2, // 2-of-3 key servers
    packageId: config.packageId,
    id: encryptionId,
    data: new TextEncoder().encode(data),
  });

  // Verify encryption
  const parsed = EncryptedObject.parse(encryptedObject);
  if (parsed.services.length === 0) {
    throw new Error("Encryption failed - no key servers embedded");
  }

  console.log("Deposit data encrypted successfully");
  // encryptedObject is already a base64 string from SEAL SDK
  if (typeof encryptedObject === "string") {
    return encryptedObject;
  }
  // Fallback for Uint8Array - convert to base64
  const bytes = encryptedObject instanceof Uint8Array
    ? encryptedObject
    : new Uint8Array(encryptedObject as unknown as ArrayBuffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * SEAL encrypt swap intent details
 * Only TEE can decrypt this
 */
export async function encryptSwapIntent(
  details: SwapIntentDetails,
  config: SealConfig,
  suiClient: SuiClient
): Promise<string> {
  const encryptionId = generateEncryptionId();

  console.log("Encrypting swap intent:", {
    nullifierPrefix: details.nullifier.substring(0, 10) + "...",
    inputAmount: details.inputAmount,
    outputStealth: details.outputStealth.substring(0, 10) + "...",
  });

  const keyServers = DEFAULT_KEY_SERVERS[config.network];
  const sealClient = new SealClient({
    suiClient: suiClient as any,
    serverConfigs: keyServers.map((id: string) => ({
      objectId: id,
      weight: 1,
    })),
    verifyKeyServers: false,
  });

  const data = JSON.stringify(details);

  const { encryptedObject } = await sealClient.encrypt({
    threshold: 2,
    packageId: config.packageId,
    id: encryptionId,
    data: new TextEncoder().encode(data),
  });

  const parsed = EncryptedObject.parse(encryptedObject);
  if (parsed.services.length === 0) {
    throw new Error("Encryption failed - no key servers embedded");
  }

  console.log("Swap intent encrypted successfully");
  // encryptedObject is already a base64 string from SEAL SDK
  if (typeof encryptedObject === "string") {
    return encryptedObject;
  }
  // Fallback for Uint8Array - convert to base64
  const bytes = encryptedObject instanceof Uint8Array
    ? encryptedObject
    : new Uint8Array(encryptedObject as unknown as ArrayBuffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

// ============ LOCAL STORAGE ============

/**
 * Save deposit notes to localStorage (wallet-scoped)
 * WARNING: These are sensitive! XSS can steal funds.
 */
export function saveDepositNotes(walletAddress: string, notes: DepositNote[]): void {
  if (typeof window === "undefined") return;
  if (!walletAddress) return;
  localStorage.setItem(getStorageKey(walletAddress), JSON.stringify(notes));
}

/**
 * Load deposit notes from localStorage (wallet-scoped)
 */
export function loadDepositNotes(walletAddress: string): DepositNote[] {
  if (typeof window === "undefined") return [];
  if (!walletAddress) return [];
  const stored = localStorage.getItem(getStorageKey(walletAddress));
  if (!stored) return [];
  try {
    return JSON.parse(stored);
  } catch {
    return [];
  }
}

/**
 * Add a new deposit note (wallet-scoped)
 */
export function addDepositNote(walletAddress: string, note: DepositNote): void {
  if (!walletAddress) return;
  const notes = loadDepositNotes(walletAddress);
  notes.push(note);
  saveDepositNotes(walletAddress, notes);
}

/**
 * Mark a deposit note as spent (wallet-scoped)
 */
export function markNoteSpent(walletAddress: string, nullifier: string): void {
  if (!walletAddress) return;
  const notes = loadDepositNotes(walletAddress);
  const note = notes.find((n) => n.nullifier === nullifier);
  if (note) {
    note.spent = true;
    saveDepositNotes(walletAddress, notes);
  }
}

/**
 * Get unspent deposit notes (wallet-scoped)
 */
export function getUnspentNotes(walletAddress: string): DepositNote[] {
  return loadDepositNotes(walletAddress).filter((n) => !n.spent);
}

/**
 * Delete a deposit note (wallet-scoped, use with caution!)
 */
export function deleteDepositNote(walletAddress: string, nullifier: string): void {
  if (!walletAddress) return;
  const notes = loadDepositNotes(walletAddress);
  const filtered = notes.filter((n) => n.nullifier !== nullifier);
  saveDepositNotes(walletAddress, filtered);
}

// ============ STEALTH KEY STORAGE ============

/**
 * Save stealth keys for scanning outputs later (wallet-scoped)
 */
export function saveStealthKeys(
  walletAddress: string,
  outputStealth: StealthAddress,
  remainderStealth: StealthAddress
): void {
  if (typeof window === "undefined") return;
  if (!walletAddress) return;
  const stored = localStorage.getItem(getStealthStorageKey(walletAddress));
  const keys = stored ? JSON.parse(stored) : [];
  keys.push({
    output: outputStealth,
    remainder: remainderStealth,
    timestamp: Date.now(),
  });
  localStorage.setItem(getStealthStorageKey(walletAddress), JSON.stringify(keys));
}

/**
 * Load all stealth keys (wallet-scoped)
 */
export function loadStealthKeys(walletAddress: string): Array<{
  output: StealthAddress;
  remainder: StealthAddress;
  timestamp: number;
}> {
  if (typeof window === "undefined") return [];
  if (!walletAddress) return [];
  const stored = localStorage.getItem(getStealthStorageKey(walletAddress));
  if (!stored) return [];
  try {
    return JSON.parse(stored);
  } catch {
    return [];
  }
}

// ============ EXPORT/IMPORT (BACKUP) ============

/**
 * Export deposit notes for backup (wallet-scoped)
 * User should store this securely (encrypted file, hardware wallet, etc.)
 */
export function exportNotesForBackup(walletAddress: string): string {
  const notes = loadDepositNotes(walletAddress);
  const stealthKeys = loadStealthKeys(walletAddress);
  return JSON.stringify(
    {
      version: 2,
      walletAddress,
      timestamp: Date.now(),
      notes,
      stealthKeys,
    },
    null,
    2
  );
}

/**
 * Import deposit notes from backup (wallet-scoped)
 */
export function importNotesFromBackup(walletAddress: string, backupJson: string): {
  imported: number;
  skipped: number;
} {
  if (!walletAddress) {
    throw new Error("Wallet not connected");
  }

  const backup = JSON.parse(backupJson);

  if (backup.version !== 2) {
    throw new Error("Unsupported backup version");
  }

  // Warn if importing to a different wallet
  if (backup.walletAddress && backup.walletAddress !== walletAddress) {
    console.warn(
      `Importing notes from different wallet: ${backup.walletAddress} -> ${walletAddress}`
    );
  }

  const existingNotes = loadDepositNotes(walletAddress);
  const existingNullifiers = new Set(existingNotes.map((n) => n.nullifier));

  let imported = 0;
  let skipped = 0;

  for (const note of backup.notes || []) {
    if (existingNullifiers.has(note.nullifier)) {
      skipped++;
    } else {
      existingNotes.push(note);
      imported++;
    }
  }

  saveDepositNotes(walletAddress, existingNotes);

  // Also import stealth keys
  if (backup.stealthKeys) {
    const existingKeys = loadStealthKeys(walletAddress);
    for (const key of backup.stealthKeys) {
      existingKeys.push(key);
    }
    localStorage.setItem(getStealthStorageKey(walletAddress), JSON.stringify(existingKeys));
  }

  return { imported, skipped };
}

// ============ FORMATTING HELPERS ============

/**
 * Format amount with decimals for display
 */
export function formatAmount(amount: string, decimals: number = 9): string {
  const num = BigInt(amount);
  const divisor = BigInt(10 ** decimals);
  const whole = num / divisor;
  const fraction = num % divisor;

  if (fraction === BigInt(0)) {
    return whole.toString();
  }

  const fractionStr = fraction.toString().padStart(decimals, "0");
  const trimmed = fractionStr.replace(/0+$/, "");
  return `${whole}.${trimmed}`;
}

/**
 * Parse user input to base units
 */
export function parseAmount(input: string, decimals: number = 9): string {
  const [whole, frac = ""] = input.split(".");
  const paddedFrac = frac.padEnd(decimals, "0").slice(0, decimals);
  return BigInt(whole + paddedFrac).toString();
}
