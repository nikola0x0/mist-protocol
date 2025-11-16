/**
 * SEAL Vault Encryption/Decryption Library
 *
 * Based on reference code from ref/seal-encryption-fe/encrypt.md
 * Implements SEAL threshold encryption for vault balances
 */

import {
  SealClient,
  getAllowlistedKeyServers,
  SessionKey,
  EncryptedObject,
} from "@mysten/seal";
import { SuiClient } from "@mysten/sui/client";
import { Transaction } from "@mysten/sui/transactions";
import { fromHex, toHex } from "@mysten/sui/utils";

export interface SealVaultConfig {
  packageId: string;
  vaultObjectId: string;
  enclaveObjectId: string;
  network: "testnet" | "mainnet";
}

/**
 * Encrypt a balance amount for vault storage
 *
 * @param amount - Amount to encrypt (as string, e.g. "100000000")
 * @param config - Vault configuration
 * @param suiClient - Sui client instance
 * @returns Encrypted object (balance_pointer)
 */
export async function encryptVaultBalance(
  amount: string,
  config: SealVaultConfig,
  suiClient: SuiClient
): Promise<string> {
  // Step 1: Generate encryption ID (vault namespace + nonce)
  // This follows the pattern: [vault_id][nonce]
  const nonce = crypto.getRandomValues(new Uint8Array(5));
  const vaultBytes = fromHex(config.vaultObjectId);
  const encryptionId = toHex(new Uint8Array([...vaultBytes, ...nonce]));

  console.log("🔐 Encrypting balance:", {
    amount,
    encryptionId: encryptionId.substring(0, 20) + "...",
  });

  // Step 2: Initialize SEAL client
  const sealClient = new SealClient({
    suiClient: suiClient as any,
    serverConfigs: getAllowlistedKeyServers(config.network).map((id) => ({
      objectId: id,
      weight: 1,
    })),
    verifyKeyServers: false,
  });

  // Step 3: Encrypt the amount
  const { encryptedObject } = await sealClient.encrypt({
    threshold: 2, // 2-of-3 key servers needed for decryption
    packageId: config.packageId,
    id: encryptionId,
    data: new TextEncoder().encode(amount),
  });

  // Step 4: Verify encryption worked
  const parsed = EncryptedObject.parse(encryptedObject);
  if (parsed.services.length === 0) {
    throw new Error("Encryption failed - no key servers embedded");
  }

  console.log(
    "✅ Encrypted successfully, key servers:",
    parsed.services.length
  );

  return encryptedObject;
}

/**
 * Decrypt a vault balance (user can view their own balance)
 *
 * @param encryptedBalance - The encrypted balance_pointer
 * @param config - Vault configuration
 * @param suiClient - Sui client instance
 * @param userAddress - User's Sui address
 * @param signPersonalMessage - Function to sign personal message (from dapp-kit)
 * @returns Decrypted amount as string
 */
export async function decryptVaultBalance(
  encryptedBalance: string,
  config: SealVaultConfig,
  suiClient: SuiClient,
  userAddress: string,
  signPersonalMessage: (
    args: {
      message: Uint8Array;
    },
    callbacks: {
      onSuccess: (result: { signature: string }) => void;
      onError: (error: any) => void;
    }
  ) => void
): Promise<string> {
  console.log("🔓 Decrypting balance for user:", userAddress);

  // Step 1: Initialize SEAL client
  const sealClient = new SealClient({
    suiClient: suiClient as any,
    serverConfigs: getAllowlistedKeyServers(config.network).map((id) => ({
      objectId: id,
      weight: 1,
    })),
    verifyKeyServers: false,
  });

  // Step 2: Create session key (user signs once for 10 min)
  const sessionKey = await SessionKey.create({
    address: userAddress,
    packageId: config.packageId,
    ttlMin: 10, // Valid for 10 minutes
    suiClient: suiClient as any,
  });

  const personalMessage = sessionKey.getPersonalMessage();

  // Step 3: Sign personal message (triggers wallet popup)
  await new Promise<void>((resolve, reject) => {
    signPersonalMessage(
      { message: personalMessage },
      {
        onSuccess: async (result: { signature: string }) => {
          try {
            await sessionKey.setPersonalMessageSignature(result.signature);
            resolve();
          } catch (error) {
            reject(error);
          }
        },
        onError: (error) => {
          reject(error);
        },
      }
    );
  });

  // Step 4: Parse encryption ID from encrypted object
  const parsed = EncryptedObject.parse(encryptedBalance);
  const encryptionId = parsed.id;

  console.log("   Encryption ID:", encryptionId.substring(0, 20) + "...");

  // Step 5: Build seal_approve transaction
  const tx = new Transaction();
  tx.moveCall({
    target: `${config.packageId}::seal_policy::seal_approve`,
    arguments: [
      tx.pure.vector("u8", Array.from(fromHex(encryptionId))),
      tx.object(config.vaultObjectId),
      tx.object(config.enclaveObjectId),
    ],
  });

  // Step 6: Sign and get transaction bytes
  const signedTx = await sessionKey.signTransaction(tx);

  // Step 7: Call SEAL key servers to decrypt
  const decrypted = await sealClient.decrypt({
    encryptedObject: encryptedBalance,
    signedTransaction: signedTx,
  });

  // Step 8: Decode plaintext
  const amount = new TextDecoder().decode(decrypted);

  console.log("✅ Decrypted amount:", amount);

  return amount;
}

/**
 * Format encrypted balance for display
 * Shows first/last few characters with "..." in middle
 */
export function formatEncryptedBalance(
  encrypted: string,
  length: number = 20
): string {
  if (encrypted.length <= length) return encrypted;
  const start = encrypted.substring(0, length / 2);
  const end = encrypted.substring(encrypted.length - length / 2);
  return `${start}...${end}`;
}

/**
 * Format amount with decimals
 * @param amount - Amount in base units (string)
 * @param decimals - Number of decimals (default 9 for SUI)
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
