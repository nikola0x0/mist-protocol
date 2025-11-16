"use client";

import { useState, useEffect } from "react";
import { ConnectButton } from "@/components/ConnectButton";
import {
  useCurrentAccount,
  useSuiClient,
  useSignAndExecuteTransaction,
  useSignPersonalMessage,
} from "@mysten/dapp-kit";
import { Transaction } from "@mysten/sui/transactions";
import { SealClient, SessionKey, EncryptedObject } from "@mysten/seal";
import { fromHex, toHex } from "@mysten/sui/utils";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";

export default function SealTestPage() {
  const account = useCurrentAccount();
  const suiClient = useSuiClient();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();
  const { mutate: signPersonalMessage } = useSignPersonalMessage();

  const [amount, setAmount] = useState("100000000");
  const [vaultId, setVaultId] = useState("");
  const [enclaveId, setEnclaveId] = useState("");
  const [encryptedData, setEncryptedData] = useState("");
  const [decryptedAmount, setDecryptedAmount] = useState("");
  const [loading, setLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [sessionKey, setSessionKey] = useState<SessionKey | null>(null);

  const packageId = process.env.NEXT_PUBLIC_PACKAGE_ID!;
  const network =
    (process.env.NEXT_PUBLIC_NETWORK as "testnet" | "mainnet") || "testnet";
  const backendUrl =
    process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

  // Create SuiClient directly for SEAL (dapp-kit client doesn't work with SEAL)
  const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

  // SEAL client initialized once
  const serverObjectIds = [
    "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75",
    "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8",
  ];

  const sealClient = new SealClient({
    suiClient: sealSuiClient,
    serverConfigs: serverObjectIds.map((id) => ({
      objectId: id,
      weight: 1,
    })),
    verifyKeyServers: false,
  });

  const addLog = (message: string) => {
    setLogs((prev) => [
      ...prev,
      `[${new Date().toLocaleTimeString()}] ${message}`,
    ]);
  };

  // Create vault
  const handleCreateVault = async () => {
    if (!account || !packageId) {
      addLog("❌ Please connect wallet and set PACKAGE_ID");
      return;
    }

    try {
      setLoading(true);
      addLog("🗄️ Creating vault...");

      const tx = new Transaction();
      tx.moveCall({
        target: `${packageId}::seal_policy::create_vault_entry`,
        arguments: [],
      });

      await new Promise<void>((resolve, reject) => {
        signAndExecute(
          {
            transaction: tx,
            options: {
              showObjectChanges: true,
              showEffects: true,
            },
          },
          {
            onSuccess: (result: any) => {
              const vaultObj = result.objectChanges?.find(
                (obj: any) => obj.type === "created" && obj.owner?.Shared
              );

              if (vaultObj) {
                const id = vaultObj.objectId;
                setVaultId(id);
                addLog(`✅ Vault created: ${id}`);
                addLog(`   Owner: ${account.address}`);
                resolve();
              } else {
                reject(new Error("Could not find vault object"));
              }
            },
            onError: (error) => {
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`❌ Vault creation failed: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // Encrypt with real SEAL
  const handleEncrypt = async () => {
    if (!sealClient || !vaultId) {
      addLog("❌ Please create vault first");
      return;
    }

    try {
      setLoading(true);
      addLog("🔐 Encrypting with SEAL...");

      // Generate encryption ID (vault namespace + nonce)
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const vaultBytes = fromHex(vaultId);
      const encryptionId = toHex(new Uint8Array([...vaultBytes, ...nonce]));

      addLog(`   Encryption ID: ${encryptionId.substring(0, 40)}...`);

      // Encrypt with SEAL
      const { encryptedObject } = await sealClient.encrypt({
        threshold: 2,
        packageId,
        id: encryptionId,
        data: new TextEncoder().encode(amount),
      });

      // Verify
      const parsed = EncryptedObject.parse(encryptedObject);
      if (parsed.services.length === 0) {
        throw new Error("Encryption failed - no key servers");
      }

      setEncryptedData(encryptedObject);
      addLog(`✅ Encrypted successfully!`);
      addLog(`   Key servers: ${parsed.services.length}`);
      addLog(`   Length: ${encryptedObject.length} chars`);
    } catch (error: any) {
      addLog(`❌ Encryption failed: ${error.message || error}`);
      console.error("Encryption error:", error);
    } finally {
      setLoading(false);
    }
  };

  // Decrypt with real SEAL (user side)
  const handleDecrypt = async () => {
    if (!sealClient || !encryptedData || !vaultId) {
      addLog("❌ Missing required data (vault, encrypted data)");
      return;
    }

    try {
      setLoading(true);
      addLog("🔓 Decrypting with SEAL (user)...");

      // Create session key if not exists
      let sk = sessionKey;
      if (!sk) {
        addLog("   Creating session key...");
        sk = await SessionKey.create({
          address: account!.address,
          packageId,
          ttlMin: 10,
          suiClient: sealSuiClient,
        });

        const personalMessage = sk.getPersonalMessage();
        addLog("   📝 Requesting signature...");

        await new Promise<void>((resolve, reject) => {
          signPersonalMessage(
            { message: personalMessage },
            {
              onSuccess: async (result: { signature: string }) => {
                try {
                  await sk!.setPersonalMessageSignature(result.signature);
                  setSessionKey(sk);
                  addLog("   ✅ Session key created (valid 10 min)");
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
      }

      // Parse encryption ID
      const parsed = EncryptedObject.parse(encryptedData);
      const encryptionId = parsed.id;
      addLog(`   Encryption ID: ${encryptionId.substring(0, 40)}...`);

      // Build seal_approve transaction (user-only version - no enclave needed!)
      const tx = new Transaction();
      tx.moveCall({
        target: `${packageId}::seal_policy::seal_approve_user`,
        arguments: [
          tx.pure.vector("u8", Array.from(fromHex(encryptionId))),
          tx.object(vaultId),
        ],
      });

      addLog("   Building seal_approve_user transaction...");

      // Sign with session key
      const signedTx = await sk.signTransaction(tx);
      addLog("   ✅ Transaction signed");

      // Call SEAL key servers
      addLog("   Calling SEAL key servers...");
      const decrypted = await sealClient.decrypt({
        encryptedObject: encryptedData,
        signedTransaction: signedTx,
      });

      // Decode
      const decryptedStr = new TextDecoder().decode(decrypted);
      setDecryptedAmount(decryptedStr);

      addLog(`✅ Decrypted: ${decryptedStr}`);
      addLog(`   Original: ${amount}, Decrypted: ${decryptedStr}`);

      if (amount === decryptedStr) {
        addLog("🎉 Perfect match!");
      }
    } catch (error: any) {
      addLog(`❌ Decryption failed: ${error.message || error}`);
      console.error("Decryption error:", error);
    } finally {
      setLoading(false);
    }
  };

  // Full round-trip test
  const handleRoundTrip = async () => {
    setLogs([]);
    addLog("🚀 Starting SEAL round-trip test (user flow)...");
    addLog("");

    if (!vaultId) {
      addLog("❌ Please create vault first");
      return;
    }

    await handleEncrypt();
    await new Promise((r) => setTimeout(r, 1000));

    if (encryptedData) {
      await handleDecrypt();
    }

    addLog("");
    addLog("✅ Round-trip complete!");
  };

  return (
    <div className="min-h-screen flex flex-col bg-[#0a0a0a]">
      <header className="border-b border-[#262626] bg-[#0a0a0a]/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="container mx-auto px-6 py-4 flex justify-between items-center">
          <div>
            <h1 className="text-2xl font-bold gradient-text">Mist Protocol</h1>
            <p className="text-xs text-gray-500 mt-1">
              Real SEAL Encryption Test
            </p>
          </div>
          <ConnectButton />
        </div>
      </header>

      <main className="flex-1 container mx-auto px-6 py-8">
        <div className="max-w-5xl mx-auto space-y-6">
          {/* Deployment Info */}
          <div className="card p-6 bg-green-950/20 border-green-800/30">
            <h2 className="text-lg font-bold mb-2 text-green-400">
              ✅ Contract Deployed!
            </h2>
            <div className="space-y-2 text-sm font-mono">
              <div>
                <span className="text-gray-500">Package ID:</span>
                <span className="ml-2 text-green-400">{packageId}</span>
              </div>
              <div>
                <span className="text-gray-500">Network:</span>
                <span className="ml-2 text-green-400">{network}</span>
              </div>
              <a
                href={`https://testnet.suivision.xyz/package/${packageId}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-400 hover:underline text-xs"
              >
                View on Explorer →
              </a>
            </div>
          </div>

          {/* Setup Section */}
          <div className="card p-6">
            <h3 className="text-lg font-bold mb-4">🔧 Setup</h3>

            {!account && (
              <div className="bg-yellow-950/20 border border-yellow-800/30 rounded p-4 mb-4">
                <p className="text-yellow-400 text-sm">
                  Please connect wallet to continue
                </p>
              </div>
            )}

            <div className="space-y-4">
              {/* Vault ID */}
              <div>
                <label className="block text-sm text-gray-400 mb-2">
                  Vault ID{" "}
                  {!vaultId && <span className="text-red-400">(required)</span>}
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={vaultId}
                    onChange={(e) => setVaultId(e.target.value)}
                    placeholder="0x... (or create new vault)"
                    className="flex-1 bg-[#0a0a0a] border border-[#262626] rounded-lg px-4 py-3 text-white font-mono text-sm"
                  />
                  <button
                    onClick={handleCreateVault}
                    disabled={loading || !account}
                    className="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 text-white rounded-lg font-medium"
                  >
                    Create Vault
                  </button>
                </div>
                <p className="text-xs text-gray-500 mt-1">
                  Your vault namespace for SEAL encryption
                </p>
              </div>

              {/* Enclave ID - Optional for testing */}
              <div>
                <label className="block text-sm text-gray-400 mb-2">
                  Enclave ID{" "}
                  <span className="text-gray-500">
                    (optional - for TEE testing only)
                  </span>
                </label>
                <input
                  type="text"
                  value={enclaveId}
                  onChange={(e) => setEnclaveId(e.target.value)}
                  placeholder="0x... (leave blank to test user decryption)"
                  className="w-full bg-[#0a0a0a] border border-[#262626] rounded-lg px-4 py-3 text-white font-mono text-sm"
                />
                <p className="text-xs text-gray-500 mt-1">
                  Not needed for user decryption! Only for TEE testing.
                </p>
              </div>
            </div>
          </div>

          {/* Test Controls */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="card p-6 space-y-4">
              <h3 className="text-lg font-bold">Test Controls</h3>

              {/* Amount Input */}
              <div>
                <label className="block text-sm text-gray-400 mb-2">
                  Amount to Encrypt
                </label>
                <input
                  type="text"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="100000000"
                  className="w-full bg-[#0a0a0a] border border-[#262626] rounded-lg px-4 py-3 text-white font-mono"
                  disabled={loading}
                />
                <p className="text-xs text-gray-500 mt-1">
                  Example: 100000000 = 100 SUI
                </p>
              </div>

              {/* Buttons */}
              <div className="space-y-2">
                <button
                  onClick={handleEncrypt}
                  disabled={loading || !vaultId || !sealClient}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:text-gray-500 text-white rounded-lg font-medium"
                >
                  🔐 Encrypt with SEAL
                </button>

                <button
                  onClick={handleDecrypt}
                  disabled={
                    loading || !encryptedData || !vaultId || !sealClient
                  }
                  className="w-full px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 disabled:text-gray-500 text-white rounded-lg font-medium"
                >
                  🔓 Decrypt with SEAL (User)
                </button>
              </div>

              <div className="pt-4 border-t border-[#262626]">
                <button
                  onClick={handleRoundTrip}
                  disabled={loading || !vaultId || !sealClient}
                  className="w-full px-4 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 disabled:text-gray-500 text-white rounded-lg font-bold"
                >
                  🚀 Run Full Round-Trip Test
                </button>
              </div>

              <button
                onClick={() => {
                  setEncryptedData("");
                  setDecryptedAmount("");
                  setLogs([]);
                }}
                disabled={loading}
                className="w-full px-4 py-2 bg-[#141414] hover:bg-[#1a1a1a] text-gray-400 rounded-lg font-medium"
              >
                Clear
              </button>
            </div>

            {/* Data Display */}
            <div className="card p-6 space-y-4">
              <h3 className="text-lg font-bold">Data Flow</h3>

              <div>
                <div className="text-xs text-gray-500 mb-1">
                  Original Amount
                </div>
                <div className="bg-[#0a0a0a] border border-[#262626] rounded p-3 text-sm font-mono text-green-400 break-all">
                  {amount || "(empty)"}
                </div>
              </div>

              <div>
                <div className="text-xs text-gray-500 mb-1">SEAL Encrypted</div>
                <div className="bg-[#0a0a0a] border border-[#262626] rounded p-3 text-sm font-mono text-blue-400 break-all h-32 overflow-y-auto">
                  {encryptedData || "(not encrypted yet)"}
                </div>
              </div>

              <div>
                <div className="text-xs text-gray-500 mb-1">
                  Decrypted Amount
                </div>
                <div className="bg-[#0a0a0a] border border-[#262626] rounded p-3 text-sm font-mono text-purple-400 break-all">
                  {decryptedAmount || "(not decrypted yet)"}
                  {decryptedAmount && amount === decryptedAmount && (
                    <span className="ml-2 text-green-400">✓ Match!</span>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* Logs */}
          <div className="card p-6">
            <h3 className="text-lg font-bold mb-4">Execution Logs</h3>
            <div className="bg-[#0a0a0a] border border-[#262626] rounded p-4 h-64 overflow-y-auto font-mono text-xs space-y-1">
              {logs.length === 0 ? (
                <div className="text-gray-600">
                  No logs yet. Connect wallet to start.
                </div>
              ) : (
                logs.map((log, i) => (
                  <div key={i} className="text-gray-300">
                    {log}
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Instructions */}
          <div className="card p-4 bg-blue-950/20 border-blue-800/30">
            <div className="flex items-start gap-3">
              <div className="text-2xl">📖</div>
              <div className="flex-1 text-sm space-y-2">
                <div className="font-bold text-blue-400">How to Test:</div>
                <ol className="list-decimal list-inside space-y-1 text-gray-400">
                  <li>Connect your Sui wallet</li>
                  <li>Click "Create Vault"</li>
                  <li>Enter amount to encrypt</li>
                  <li>Click "Encrypt with SEAL"</li>
                  <li>Click "Decrypt with SEAL" (sign once, valid 10 min)</li>
                  <li>Verify decrypted amount matches!</li>
                </ol>
                <div className="pt-2 border-t border-blue-800/30 mt-3">
                  <span className="text-blue-400 font-medium">Note:</span>
                  <span className="text-gray-400 ml-2">
                    This uses real SEAL encryption with testnet key servers!
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>

      <footer className="border-t border-[#262626] py-8">
        <div className="container mx-auto px-6 text-center text-sm text-gray-500">
          Real SEAL Integration • Mist Protocol
        </div>
      </footer>
    </div>
  );
}
