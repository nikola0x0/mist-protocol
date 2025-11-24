"use client";

import { useState, useEffect } from "react";
import { Transaction } from "@mysten/sui/transactions";
import {
  useSignAndExecuteTransaction,
  useCurrentAccount,
  useSignPersonalMessage,
} from "@mysten/dapp-kit";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { SealClient, SessionKey, EncryptedObject } from "@mysten/seal";
import { fromHex, toHex } from "@mysten/sui/utils";

// Token icons
const TOKEN_ICONS = {
  SUI: "https://s2.coinmarketcap.com/static/img/coins/64x64/20947.png",
  USDC: "https://s2.coinmarketcap.com/static/img/coins/64x64/3408.png",
};

// Types for our ticket vault system
interface Ticket {
  ticket_id: number;
  token_type: "SUI" | "USDC";
  encrypted_amount: string;
}

interface Vault {
  id: string;
  owner: string;
  tickets: Ticket[];
  next_ticket_id: number;
  availableVaults: string[]; // All vault IDs user has access to
}

export function UnwrapCard() {
  const [vault, setVault] = useState<Vault | null>(null);
  const [selectedTicketId, setSelectedTicketId] = useState<number | null>(null);
  const [unwrapAmount, setUnwrapAmount] = useState("");
  const [decryptedAmount, setDecryptedAmount] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [decrypting, setDecrypting] = useState(false);
  const [loadingVault, setLoadingVault] = useState(false);
  const [sealClient, setSealClient] = useState<SealClient | null>(null);
  const [sessionKey, setSessionKey] = useState<SessionKey | null>(null);
  const { mutateAsync: signAndExecuteTransaction } =
    useSignAndExecuteTransaction();
  const { mutate: signPersonalMessage } = useSignPersonalMessage();
  const currentAccount = useCurrentAccount();

  const client = new SuiClient({
    url:
      process.env.NEXT_PUBLIC_NETWORK === "mainnet"
        ? "https://fullnode.mainnet.sui.io"
        : "https://fullnode.testnet.sui.io",
  });

  // Initialize SEAL client (same as mist-flow-test)
  const initializeSealClient = async () => {
    if (sealClient) return; // Already initialized

    try {
      console.log("🔐 Initializing SEAL client...");

      // Create a fresh SuiClient for SEAL
      const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

      const server1 =
        process.env.NEXT_PUBLIC_SEAL_SERVER_1 ||
        "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75";
      const server2 =
        process.env.NEXT_PUBLIC_SEAL_SERVER_2 ||
        "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8";

      const client = new SealClient({
        suiClient: sealSuiClient,
        serverConfigs: [
          { objectId: server1, weight: 1 },
          { objectId: server2, weight: 1 },
        ],
        verifyKeyServers: false,
      });

      setSealClient(client);
      console.log("✅ SEAL client initialized");
    } catch (error: any) {
      console.error("❌ SEAL client init failed:", error.message);
    }
  };

  // Discover user's vaults automatically
  const discoverVaults = async () => {
    if (!currentAccount || !process.env.NEXT_PUBLIC_PACKAGE_ID) {
      console.log("No account or package ID");
      return [];
    }

    try {
      console.log("🔍 Discovering user vaults...");

      // Query user's VaultRegistry objects
      const registries = await client.getOwnedObjects({
        owner: currentAccount.address,
        filter: {
          StructType: `${process.env.NEXT_PUBLIC_PACKAGE_ID}::seal_policy::VaultRegistry`,
        },
        options: { showContent: true },
      });

      if (registries.data.length === 0) {
        console.log("📭 No vault registries found");
        return [];
      }

      console.log(`📋 Found ${registries.data.length} registry/registries`);

      // Collect all vault IDs from ALL registries
      const allVaultIds: string[] = [];

      for (const registry of registries.data) {
        const registryObjectId = registry.data?.objectId;
        if (!registryObjectId) continue;

        // Extract vault IDs from this registry
        const registryDetails = await client.getObject({
          id: registryObjectId,
          options: { showContent: true },
        });

        if (registryDetails.data?.content?.dataType === "moveObject") {
          const fields = registryDetails.data.content.fields as any;
          const vaultIds = fields.vault_ids || [];
          allVaultIds.push(...vaultIds);
          console.log(`📦 Registry has ${vaultIds.length} vault(s)`);
        }
      }

      console.log(`✅ Found ${allVaultIds.length} total vault(s)`);
      return allVaultIds;
    } catch (error) {
      console.error("Failed to discover vaults:", error);
      return [];
    }
  };

  // Load user's vault tickets
  const loadVault = async () => {
    if (!currentAccount) return;

    try {
      // First try to discover vaults automatically
      const vaultIds = await discoverVaults();

      // If no vaults found, try environment variable as fallback
      const vaultId =
        vaultIds.length > 0 ? vaultIds[0] : process.env.NEXT_PUBLIC_VAULT_ID;

      if (!vaultId) {
        console.log("No vaults found and no fallback vault ID");
        return;
      }

      console.log(`📦 Loading vault: ${vaultId.substring(0, 20)}...`);

      const vaultObject = await client.getObject({
        id: vaultId,
        options: {
          showContent: true,
          showOwner: true,
        },
      });

      if (!vaultObject.data?.content) {
        console.log("Vault not found");
        return;
      }

      const content = vaultObject.data.content as any;
      const tickets: Ticket[] = [];

      // Load tickets properly using getDynamicFieldObject (like mist-flow-test)
      const nextTicketId = content.fields.next_ticket_id || 0;
      const objectBagId = content.fields.tickets.fields.id.id;

      console.log(`📂 ObjectBag ID: ${objectBagId.substring(0, 20)}...`);
      console.log(`🎫 Expected ${nextTicketId} tickets`);

      for (let i = 0; i < nextTicketId; i++) {
        try {
          // Query dynamic field (ticket in ObjectBag)
          const ticketField = await client.getDynamicFieldObject({
            parentId: objectBagId,
            name: {
              type: "u64",
              value: i.toString(),
            },
          });

          if (!ticketField.data) {
            console.log(
              `  ⚠️ Ticket #${i} not found (may be locked in pending swap intent)`
            );
            continue;
          }

          if (ticketField.data?.content?.dataType === "moveObject") {
            const ticketFields = ticketField.data.content.fields as any;

            // Handle both direct fields and nested value fields
            const encryptedAmountBytes =
              ticketFields.encrypted_amount ||
              ticketFields.value?.fields?.encrypted_amount;
            const tokenType =
              ticketFields.token_type || ticketFields.value?.fields?.token_type;
            const ticketId =
              ticketFields.ticket_id || ticketFields.value?.fields?.ticket_id;

            if (!encryptedAmountBytes) {
              console.log(`  ⚠️ Ticket #${i} has no encrypted_amount field`);
              continue;
            }

            const encryptedAmountHex =
              Buffer.from(encryptedAmountBytes).toString("hex");

            tickets.push({
              ticket_id: ticketId !== undefined ? ticketId : i,
              token_type: tokenType || "SUI", // Default to SUI if unknown
              encrypted_amount: `0x${encryptedAmountHex}`,
            });

            console.log(`  ✅ Loaded Ticket #${i} (${tokenType})`);
          }
        } catch (err) {
          console.log(
            `  ⚠️ Ticket #${i} not found (may be locked in swap intent)`
          );
        }
      }

      const vault = {
        id: vaultId,
        owner: content.fields.owner,
        tickets,
        next_ticket_id: content.fields.next_ticket_id,
        availableVaults: vaultIds, // Store all discovered vaults
      };

      setVault(vault);
      console.log(`✅ Loaded vault with ${tickets.length} tickets`);
    } catch (error) {
      console.error("Failed to load vault:", error);
    }
  };

  // Load a specific vault by ID
  const loadSpecificVault = async (vaultId: string) => {
    if (!currentAccount) return;

    try {
      setLoadingVault(true);
      console.log(`📦 Loading specific vault: ${vaultId.substring(0, 20)}...`);

      const vaultObject = await client.getObject({
        id: vaultId,
        options: {
          showContent: true,
          showOwner: true,
        },
      });

      if (!vaultObject.data?.content) {
        console.log("Vault not found");
        return;
      }

      const content = vaultObject.data.content as any;
      const tickets: Ticket[] = [];

      // Load tickets properly using getDynamicFieldObject (like mist-flow-test)
      const nextTicketId = content.fields.next_ticket_id || 0;
      const objectBagId = content.fields.tickets.fields.id.id;

      console.log(`📂 ObjectBag ID: ${objectBagId.substring(0, 20)}...`);
      console.log(`🎫 Expected ${nextTicketId} tickets`);

      for (let i = 0; i < nextTicketId; i++) {
        try {
          // Query dynamic field (ticket in ObjectBag)
          const ticketField = await client.getDynamicFieldObject({
            parentId: objectBagId,
            name: {
              type: "u64",
              value: i.toString(),
            },
          });

          if (!ticketField.data) {
            console.log(
              `  ⚠️ Ticket #${i} not found (may be locked in pending swap intent)`
            );
            continue;
          }

          if (ticketField.data?.content?.dataType === "moveObject") {
            const ticketFields = ticketField.data.content.fields as any;

            // Handle both direct fields and nested value fields
            const encryptedAmountBytes =
              ticketFields.encrypted_amount ||
              ticketFields.value?.fields?.encrypted_amount;
            const tokenType =
              ticketFields.token_type || ticketFields.value?.fields?.token_type;
            const ticketId =
              ticketFields.ticket_id || ticketFields.value?.fields?.ticket_id;

            if (!encryptedAmountBytes) {
              console.log(`  ⚠️ Ticket #${i} has no encrypted_amount field`);
              continue;
            }

            const encryptedAmountHex =
              Buffer.from(encryptedAmountBytes).toString("hex");

            tickets.push({
              ticket_id: ticketId !== undefined ? ticketId : i,
              token_type: tokenType || "SUI", // Default to SUI if unknown
              encrypted_amount: `0x${encryptedAmountHex}`,
            });

            console.log(`  ✅ Loaded Ticket #${i} (${tokenType})`);
          }
        } catch (err) {
          console.log(
            `  ⚠️ Ticket #${i} not found (may be locked in swap intent)`
          );
        }
      }

      // Get all vault IDs again for the dropdown
      const allVaultIds = await discoverVaults();

      const updatedVault = {
        id: vaultId,
        owner: content.fields.owner,
        tickets,
        next_ticket_id: content.fields.next_ticket_id,
        availableVaults: allVaultIds,
      };

      setVault(updatedVault);
      console.log(`✅ Loaded vault with ${tickets.length} tickets`);
    } catch (error) {
      console.error("Failed to load specific vault:", error);
    } finally {
      setLoadingVault(false);
    }
  };

  // Decrypt ticket amount locally using real SEAL (same as mist-flow-test)
  const decryptTicketAmount = async (ticket: Ticket) => {
    if (!currentAccount || !sealClient || !vault) {
      console.error("Missing required data for decryption");
      setDecryptedAmount("DECRYPTION_FAILED");
      return;
    }

    setDecrypting(true);
    try {
      console.log("🔓 Decrypting ticket with SEAL (user)...");
      console.log(`   Ticket ID: ${ticket.ticket_id}`);
      console.log(`   Token Type: ${ticket.token_type}`);

      // Create session key if not exists
      let sk = sessionKey;
      if (!sk) {
        console.log("   Creating session key...");
        const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

        sk = await SessionKey.create({
          address: currentAccount.address,
          packageId: process.env.NEXT_PUBLIC_PACKAGE_ID!,
          ttlMin: 10,
          suiClient: sealSuiClient,
        });

        const personalMessage = sk.getPersonalMessage();
        console.log("   ✍️ Requesting signature...");

        await new Promise<void>((resolve, reject) => {
          signPersonalMessage(
            { message: personalMessage },
            {
              onSuccess: async (result: { signature: string }) => {
                try {
                  await sk!.setPersonalMessageSignature(result.signature);
                  setSessionKey(sk);
                  console.log("   ✅ Session key created (valid 10 min)");
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

      // Parse EncryptedObject from encrypted_amount
      const encryptedBytes = fromHex(ticket.encrypted_amount);
      const parsed = EncryptedObject.parse(encryptedBytes);

      // Handle encryption ID (might be array, Uint8Array, or string)
      let encryptionId: Uint8Array;
      if (parsed.id instanceof Uint8Array) {
        encryptionId = parsed.id;
      } else if (Array.isArray(parsed.id)) {
        encryptionId = new Uint8Array(parsed.id);
      } else if (typeof parsed.id === "string") {
        encryptionId = fromHex(parsed.id);
      } else {
        throw new Error(`Unexpected encryption ID type: ${typeof parsed.id}`);
      }

      const encryptionIdHex = toHex(encryptionId);
      console.log(
        `   🔑 Encryption ID: ${encryptionIdHex.substring(0, 20)}...`
      );

      // Build seal_approve_user transaction
      const tx = new Transaction();
      tx.moveCall({
        target: `${process.env.NEXT_PUBLIC_PACKAGE_ID}::seal_policy::seal_approve_user`,
        arguments: [
          tx.pure.vector("u8", Array.from(encryptionId)),
          tx.object(vault.id),
        ],
      });

      // Create fresh SuiClient for building tx
      const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

      // Build transaction bytes
      console.log("   🔨 Building transaction bytes...");
      const txBytes = await tx.build({
        client: sealSuiClient,
        onlyTransactionKind: true,
      });

      // Call SEAL key servers with the session key
      console.log("   🔐 Calling SEAL key servers...");
      const decrypted = await sealClient.decrypt({
        data: encryptedBytes,
        sessionKey: sk,
        txBytes: txBytes,
      });

      // Decode the decrypted amount
      const decoder = new TextDecoder();
      const decryptedAmount = decoder.decode(decrypted);

      console.log(
        `   ✅ Decrypted amount: ${decryptedAmount} (${(
          parseInt(decryptedAmount) / (ticket.token_type === "SUI" ? 1e9 : 1e6)
        ).toFixed(6)} ${ticket.token_type})`
      );

      setDecryptedAmount(decryptedAmount);
    } catch (error: any) {
      console.error("❌ Decryption failed:", error.message);
      setDecryptedAmount("DECRYPTION_FAILED");
    } finally {
      setDecrypting(false);
    }
  };

  // Handle ticket selection
  const handleTicketSelect = (ticket: Ticket) => {
    setSelectedTicketId(ticket.ticket_id);
    setUnwrapAmount("");
    setDecryptedAmount("");
    decryptTicketAmount(ticket);
  };

  // Execute unwrap transaction
  const handleUnwrap = async () => {
    if (!selectedTicketId || !vault) return;

    setLoading(true);
    try {
      const ticket = vault.tickets.find(
        (t) => t.ticket_id === selectedTicketId
      );
      if (!ticket) throw new Error("Ticket not found");

      const tx = new Transaction();

      // Full unwrap only - use convenience functions
      const fullAmount = parseInt(decryptedAmount); // Get full decrypted amount

      if (ticket.token_type === "SUI") {
        tx.moveCall({
          target: `${process.env.NEXT_PUBLIC_PACKAGE_ID}::mist_protocol::unwrap_sui`,
          arguments: [
            tx.object(vault.id), // vault
            tx.object(process.env.NEXT_PUBLIC_POOL_ID || ""), // pool
            tx.pure.u64(selectedTicketId), // ticket_id
            tx.pure.u64(fullAmount), // amount
          ],
        });
      } else {
        tx.moveCall({
          target: `${process.env.NEXT_PUBLIC_PACKAGE_ID}::mist_protocol::unwrap_usdc`,
          arguments: [
            tx.object(vault.id), // vault
            tx.object(process.env.NEXT_PUBLIC_POOL_ID || ""), // pool
            tx.pure.u64(selectedTicketId), // ticket_id
            tx.pure.u64(fullAmount), // amount
          ],
        });
      }

      await signAndExecuteTransaction({
        transaction: tx,
      });

      // Refresh vault after successful unwrap
      await loadVault();
      setSelectedTicketId(null);
      setUnwrapAmount("");
      setDecryptedAmount("");

      const unwrapAmountDisplay = (
        parseInt(decryptedAmount) / (ticket.token_type === "SUI" ? 1e9 : 1e6)
      ).toFixed(6);

      alert(
        `Successfully unwrapped ${unwrapAmountDisplay} ${ticket.token_type}!`
      );
    } catch (error) {
      console.error("Unwrap failed:", error);
      alert("Unwrap failed. Please try again.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Initialize SEAL client when account is available
    if (currentAccount) {
      initializeSealClient();
      setLoadingVault(true);
      loadVault().finally(() => setLoadingVault(false));
    }
  }, [currentAccount]);

  const selectedTicket = vault?.tickets.find(
    (t) => t.ticket_id === selectedTicketId
  );

  const displayAmount = () => {
    if (!selectedTicket || !decryptedAmount) return "Loading...";

    if (decryptedAmount === "DECRYPT_NEEDED") return "Decryption required";
    if (decryptedAmount === "DECRYPTION_FAILED") return "Decryption failed";

    try {
      const amount = parseInt(decryptedAmount);
      if (isNaN(amount)) return "Invalid amount";
      return (
        amount / (selectedTicket?.token_type === "SUI" ? 1e9 : 1e6)
      ).toFixed(6);
    } catch {
      return "Parse error";
    }
  };

  return (
    <div className="card p-6 animate-slide-up">
      <h3 className="text-xl font-bold mb-6">Unwrap Tokens</h3>

      {/* Vault Selection */}
      {vault?.availableVaults && vault.availableVaults.length > 1 && (
        <div className="mb-6">
          <label className="text-sm text-gray-400 mb-2 block">
            Select Vault
          </label>
          <select
            value={vault.id}
            onChange={(e) => {
              // Load selected vault
              const selectedVaultId = e.target.value;
              loadSpecificVault(selectedVaultId);
            }}
            className="w-full px-4 py-3 bg-[#0a0a0a] border border-[#262626] rounded-lg text-white"
            disabled={loadingVault}
          >
            {vault.availableVaults.map((vaultId) => (
              <option key={vaultId} value={vaultId}>
                Vault {vaultId.substring(0, 10)}...
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Loading State */}
      {loadingVault && (
        <div className="mb-6 text-center py-8">
          <div className="text-gray-400">
            <div className="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
            <p>Discovering vaults...</p>
          </div>
        </div>
      )}

      {/* No Vaults Found */}
      {!loadingVault && !vault && (
        <div className="text-center py-8">
          <div className="text-gray-500 mb-4">
            <div className="text-4xl mb-3">🗄️</div>
            <h4 className="font-medium text-lg mb-2">No Vaults Found</h4>
            <p className="text-sm mb-4">
              You don&apos;t have any vaults with tickets yet
            </p>
          </div>
          <div className="space-y-2 text-sm text-gray-600">
            <p>To get started:</p>
            <ol className="list-decimal list-inside space-y-1 text-left max-w-xs mx-auto">
              <li>
                Go to the <strong>Wrap</strong> tab
              </li>
              <li>Deposit tokens to create encrypted tickets</li>
              <li>Come back here to unwrap them</li>
            </ol>
          </div>
        </div>
      )}

      {/* Ticket Selection */}
      {vault && !loadingVault && (
        <div className="mb-6">
          <label className="text-sm text-gray-400 mb-2 block">
            Select Ticket from {vault.id.substring(0, 10)}...
          </label>
          {vault.tickets && vault.tickets.length > 0 ? (
            <div className="space-y-2 max-h-40 overflow-y-auto">
              {vault.tickets.map((ticket) => (
                <button
                  key={ticket.ticket_id}
                  onClick={() => handleTicketSelect(ticket)}
                  className={`w-full p-3 rounded-lg text-left transition border ${
                    selectedTicketId === ticket.ticket_id
                      ? "bg-blue-600/20 border-blue-600 text-blue-400"
                      : "bg-[#141414] border-[#262626] hover:border-[#333] hover:text-white"
                  }`}
                >
                  <div className="flex justify-between items-center">
                    <div className="flex items-center gap-2">
                      <img
                        src={TOKEN_ICONS[ticket.token_type]}
                        alt={ticket.token_type}
                        className="w-5 h-5"
                      />
                      <span className="font-medium">
                        Ticket #{ticket.ticket_id}
                      </span>
                      <span className="text-sm text-gray-400">
                        {ticket.token_type}
                      </span>
                    </div>
                    {decrypting && selectedTicketId === ticket.ticket_id && (
                      <span className="text-xs text-yellow-400">
                        Decrypting...
                      </span>
                    )}
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500">
              <p>No tickets found in this vault</p>
              <p className="text-sm">
                Wrap some tokens first to create tickets
              </p>
            </div>
          )}
        </div>
      )}

      {/* Ticket Details */}
      {selectedTicket && (
        <div className="mb-6 p-4 bg-[#141414] rounded-lg border border-[#262626]">
          <div className="flex justify-between items-center mb-2">
            <span className="text-sm text-gray-400">
              Ticket #{selectedTicket.ticket_id}
            </span>
            <div className="flex items-center gap-2 px-2 py-1 bg-white/5 border border-white/10">
              <img
                src={TOKEN_ICONS[selectedTicket.token_type]}
                alt={selectedTicket.token_type}
                className="w-4 h-4"
              />
              <span className="text-xs font-medium text-gray-300">
                {selectedTicket.token_type}
              </span>
            </div>
          </div>
          <div className="text-sm text-gray-500 mb-1">
            Decrypted Amount: {displayAmount()} {selectedTicket.token_type}
          </div>
          <div className="text-xs text-gray-600">
            Encrypted: {selectedTicket.encrypted_amount.slice(0, 20)}...
          </div>
        </div>
      )}

      {/* Unwrap Button */}
      <button
        onClick={handleUnwrap}
        disabled={
          !selectedTicketId ||
          loading ||
          !decryptedAmount ||
          decryptedAmount === "DECRYPT_NEEDED" ||
          decryptedAmount === "DECRYPTION_FAILED"
        }
        className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-medium py-4 rounded-lg transition flex items-center justify-center gap-2"
      >
        {selectedTicket && (
          <img
            src={TOKEN_ICONS[selectedTicket.token_type]}
            alt={selectedTicket.token_type}
            className="w-5 h-5"
          />
        )}
        {loading
          ? "Unwrapping..."
          : `Unwrap ${selectedTicket?.token_type || "Tokens"}`}
      </button>

      {/* Info Section */}
      {selectedTicket && (
        <div className="mt-4 text-xs text-gray-500 text-center space-y-1">
          <p>ℹ️ Ticket #{selectedTicket.ticket_id} will be burned</p>
          <p>
            You&apos;ll receive {displayAmount()} {selectedTicket.token_type} in
            your wallet
          </p>
          <p className="text-yellow-500">
            ⚠️ Partial unwrap is future work (requires SEAL re-encryption)
          </p>
        </div>
      )}
    </div>
  );
}
