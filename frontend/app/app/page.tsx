"use client";

import { useState, useEffect } from "react";
import Image from "next/image";
import { ConnectButton } from "@/components/ConnectButton";
import { UnwrapCard } from "@/components/UnwrapCard";
import {
  useCurrentAccount,
  useSuiClient,
  useSignAndExecuteTransaction,
  useSignPersonalMessage,
} from "@mysten/dapp-kit";
import { Transaction } from "@mysten/sui/transactions";
import { SealClient, EncryptedObject, SessionKey } from "@mysten/seal";
import { fromHex, toHex } from "@mysten/sui/utils";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";

// Utility to sanitize hex IDs
const sanitizeHexId = (id: string): string => {
  if (!id) return "";
  return id.startsWith("0x") ? id : `0x${id}`;
};

// Token icons
const TOKEN_ICONS = {
  SUI: "https://s2.coinmarketcap.com/static/img/coins/64x64/20947.png",
  USDC: "https://s2.coinmarketcap.com/static/img/coins/64x64/3408.png",
};

export default function MistFlowTestPage() {
  const account = useCurrentAccount();
  const suiClient = useSuiClient();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();
  const { mutate: signPersonalMessage } = useSignPersonalMessage();

  // Configuration
  const packageId = process.env.NEXT_PUBLIC_PACKAGE_ID!;
  const poolId = process.env.NEXT_PUBLIC_POOL_ID!;
  const queueId = process.env.NEXT_PUBLIC_INTENT_QUEUE_ID!;
  const backendUrl =
    process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

  // State
  const [vaultId, setVaultId] = useState("");
  const [availableVaults, setAvailableVaults] = useState<string[]>([]);
  const [registryId, setRegistryId] = useState("");
  const [enclaveId, setEnclaveId] = useState(""); // Optional for now
  const [sealClient, setSealClient] = useState<SealClient | null>(null);

  // Tickets state
  const [tickets, setTickets] = useState<
    Array<{
      id: number;
      tokenType: string;
      amount: string;
      encryptedAmount: string;
      txUrl: string;
      decryptedAmount?: string; // For testing decryption
    }>
  >([]);

  // Swap state
  const [selectedTicketIds, setSelectedTicketIds] = useState<number[]>([]);
  const [swapConfig, setSwapConfig] = useState({
    tokenOut: "USDC",
    slippagePercent: "0.5",
    deadline: Math.floor(Date.now() / 1000) + 3600,
  });
  const [swapResult, setSwapResult] = useState<any>(null);
  const [pendingIntents, setPendingIntents] = useState<any[]>([]);

  // UI state
  const [loading, setLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [activePhase, setActivePhase] = useState<
    "vault" | "wrap" | "swap" | "unwrap"
  >("vault");

  const addLog = (message: string) => {
    setLogs((prev) => [
      ...prev,
      `[${new Date().toLocaleTimeString()}] ${message}`,
    ]);
  };

  // ============================================================================
  // PHASE 1: SETUP - Vault Discovery
  // ============================================================================

  const initializeSealClient = async () => {
    if (sealClient) return; // Already initialized

    try {
      addLog("Initializing SEAL client...");

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
      addLog("SEAL client initialized");
    } catch (error: any) {
      addLog(`ERROR: SEAL client init failed: ${error.message}`);
    }
  };

  const discoverVaults = async () => {
    if (!account) {
      addLog("ERROR: Please connect wallet first");
      return;
    }

    try {
      setLoading(true);
      addLog("Discovering your vaults...");

      // Query user's VaultRegistry object
      const registries = await suiClient.getOwnedObjects({
        owner: account.address,
        filter: {
          StructType: `${packageId}::seal_policy::VaultRegistry`,
        },
        options: { showContent: true },
      });

      if (registries.data.length === 0) {
        addLog("No vaults found. Create your first vault!");
        setAvailableVaults([]);
        return;
      }

      addLog(`Found ${registries.data.length} registry/registries`);

      // Collect all vault IDs from ALL registries
      const allVaultIds: string[] = [];

      for (const registry of registries.data) {
        const registryObjectId = registry.data?.objectId;
        if (!registryObjectId) continue;

        // Extract vault IDs from this registry
        const registryDetails = await suiClient.getObject({
          id: registryObjectId,
          options: { showContent: true },
        });

        if (registryDetails.data?.content?.dataType === "moveObject") {
          const fields = registryDetails.data.content.fields as any;
          const vaultIds = fields.vault_ids || [];
          allVaultIds.push(...vaultIds);
          addLog(
            `  Registry ${registryObjectId.substring(0, 20)}... has ${
              vaultIds.length
            } vault(s)`
          );
        }
      }

      // Use the first registry for adding new vaults
      const firstRegistry = registries.data[0];
      const firstRegistryId = firstRegistry.data?.objectId;
      setRegistryId(firstRegistryId || "");

      addLog(`Found ${allVaultIds.length} total vault(s)`);
      setAvailableVaults(allVaultIds);

      // Auto-select first vault and initialize SEAL client
      if (allVaultIds.length > 0) {
        setVaultId(allVaultIds[0]);
        addLog(`Selected vault: ${allVaultIds[0].substring(0, 20)}...`);

        // Initialize SEAL client if not already initialized
        await initializeSealClient();
      }
    } catch (error: any) {
      addLog(`ERROR: Discovery error: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateVault = async () => {
    if (!account) {
      addLog("ERROR: Please connect wallet first");
      return;
    }

    try {
      setLoading(true);

      // Check if user already has a registry
      const registries = await suiClient.getOwnedObjects({
        owner: account.address,
        filter: {
          StructType: `${packageId}::seal_policy::VaultRegistry`,
        },
      });

      const tx = new Transaction();

      if (registries.data.length > 0) {
        // User has a registry - add vault to existing registry
        const existingRegistryId = registries.data[0].data?.objectId;
        addLog(`Adding vault to existing registry...`);

        tx.moveCall({
          target: `${packageId}::seal_policy::add_vault_to_registry`,
          arguments: [tx.object(existingRegistryId!)],
        });
      } else {
        // User doesn't have a registry - create first vault
        addLog("Creating first vault...");

        tx.moveCall({
          target: `${packageId}::seal_policy::create_vault_entry`,
          arguments: [],
        });
      }

      await new Promise<void>((resolve, reject) => {
        signAndExecute(
          { transaction: tx },
          {
            onSuccess: async (result) => {
              addLog(`Vault creation TX: ${result.digest}`);

              // Wait for transaction to be indexed
              await new Promise((r) => setTimeout(r, 2000));

              // Get transaction details
              const txResult = await suiClient.getTransactionBlock({
                digest: result.digest,
                options: { showObjectChanges: true },
              });

              // Find the vault object
              const vaultObj = txResult.objectChanges?.find(
                (change) =>
                  change.type === "created" &&
                  change.objectType?.includes("seal_policy::VaultEntry")
              );

              if (vaultObj && vaultObj.type === "created") {
                const id = sanitizeHexId(vaultObj.objectId);
                setVaultId(id);
                addLog(`Vault created: ${id}`);

                // Initialize SEAL client
                // Create a fresh SuiClient for SEAL (required for proper initialization)
                const sealSuiClient = new SuiClient({
                  url: getFullnodeUrl("testnet"),
                });

                const server1 =
                  process.env.NEXT_PUBLIC_SEAL_SERVER_1 ||
                  "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75";
                const server2 =
                  process.env.NEXT_PUBLIC_SEAL_SERVER_2 ||
                  "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8";

                addLog(`SEAL Server 1: ${server1.substring(0, 20)}...`);
                addLog(`SEAL Server 2: ${server2.substring(0, 20)}...`);

                const client = new SealClient({
                  suiClient: sealSuiClient, // Use fresh SuiClient
                  serverConfigs: [
                    {
                      objectId: server1,
                      weight: 1,
                    },
                    {
                      objectId: server2,
                      weight: 1,
                    },
                  ],
                  verifyKeyServers: false,
                });
                setSealClient(client);
                addLog("SEAL client initialized");

                // Refresh vault discovery
                await discoverVaults();

                resolve();
              } else {
                throw new Error("Vault object not found in transaction");
              }
            },
            onError: (error) => {
              addLog(`ERROR: Vault creation failed: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`ERROR: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 2: DEPOSIT - Load & Create Tickets
  // ============================================================================

  const loadTicketsFromVault = async () => {
    if (!vaultId) {
      addLog("ERROR: Please select a vault first");
      return;
    }

    try {
      setLoading(true);
      addLog("Loading tickets from vault...");

      // Get the vault object
      const vaultObj = await suiClient.getObject({
        id: vaultId,
        options: { showContent: true },
      });

      if (vaultObj.data?.content?.dataType !== "moveObject") {
        addLog("ERROR: Invalid vault object");
        return;
      }

      const fields = vaultObj.data.content.fields as any;
      const nextTicketId = parseInt(fields.next_ticket_id);

      addLog(`Vault has ${nextTicketId} ticket(s)`);

      if (nextTicketId === 0) {
        addLog("No tickets in vault yet");
        setTickets([]);
        return;
      }

      // Load each ticket from the ObjectBag
      const loadedTickets = [];
      const objectBagId = fields.tickets.fields.id.id;

      addLog(`ObjectBag ID: ${objectBagId.substring(0, 20)}...`);

      for (let i = 0; i < nextTicketId; i++) {
        try {
          // Query dynamic field (ticket in ObjectBag)
          // Note: value must be a string representation of the u64
          const ticketField = await suiClient.getDynamicFieldObject({
            parentId: objectBagId,
            name: {
              type: "u64",
              value: i.toString(),
            },
          });

          addLog(
            `  🔍 Raw ticket field: ${JSON.stringify(
              ticketField.data?.content
            )?.substring(0, 100)}...`
          );

          if (!ticketField.data) {
            addLog(
              `  Ticket #${i} not found (may be locked in pending swap intent)`
            );
            continue;
          }

          if (ticketField.data?.content?.dataType === "moveObject") {
            const ticketFields = ticketField.data.content.fields as any;

            // The ticket is the entire object, not nested in 'value'
            const encryptedAmountBytes =
              ticketFields.encrypted_amount ||
              ticketFields.value?.fields?.encrypted_amount;
            const tokenType =
              ticketFields.token_type || ticketFields.value?.fields?.token_type;
            const ticketId =
              ticketFields.ticket_id || ticketFields.value?.fields?.ticket_id;

            if (!encryptedAmountBytes) {
              addLog(`  WARNING: Ticket #${i} has no encrypted_amount field`);
              continue;
            }

            const encryptedAmountHex = toHex(
              new Uint8Array(encryptedAmountBytes)
            );

            loadedTickets.push({
              id: ticketId !== undefined ? ticketId : i,
              tokenType: tokenType || "UNKNOWN",
              amount: "?", // We don't know the amount until decryption
              encryptedAmount: encryptedAmountHex,
              txUrl: `https://testnet.suivision.xyz/object/${ticketField.data.objectId}`,
            });

            addLog(`  Loaded Ticket #${i} (${tokenType})`);
          }
        } catch (err: any) {
          addLog(`  Ticket #${i} not found (may be locked in swap intent)`);
        }
      }

      setTickets(loadedTickets);
      addLog(`Loaded ${loadedTickets.length} tickets`);
    } catch (error: any) {
      addLog(`ERROR: Failed to load tickets: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDepositSUI = async (amount: string) => {
    if (!sealClient || !vaultId) {
      addLog("ERROR: Please create vault first");
      return;
    }

    try {
      setLoading(true);
      addLog(
        `Depositing ${amount} MIST (${(
          parseInt(amount) / 1_000_000_000
        ).toFixed(4)} SUI)...`
      );

      // Step 1: Encrypt the amount with SEAL
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const cleanVaultId = sanitizeHexId(vaultId);
      const vaultBytes = fromHex(cleanVaultId);
      const combined = new Uint8Array(vaultBytes.length + nonce.length);
      combined.set(vaultBytes, 0);
      combined.set(nonce, vaultBytes.length);
      const encryptionId = toHex(combined);

      addLog(`   Encrypting amount with SEAL...`);
      addLog(`   Package ID: ${packageId.substring(0, 20)}...`);
      addLog(`   Encryption ID: ${encryptionId.substring(0, 40)}...`);

      let encryptedObject;
      try {
        const result = await sealClient.encrypt({
          threshold: 2,
          packageId,
          id: encryptionId,
          data: new TextEncoder().encode(amount),
        });
        encryptedObject = result.encryptedObject;
      } catch (encryptError: any) {
        addLog(`ERROR: SEAL encryption error: ${encryptError.message}`);
        addLog(`   Details: ${JSON.stringify(encryptError)}`);
        throw encryptError;
      }

      const encryptedPointer = Array.from(encryptedObject);
      addLog(`Amount encrypted (${encryptedPointer.length} bytes)`);

      // Step 2: Call wrap_sui to create ticket
      const tx = new Transaction();
      const [coin] = tx.splitCoins(tx.gas, [parseInt(amount)]);

      tx.moveCall({
        target: `${packageId}::mist_protocol::wrap_sui`,
        arguments: [
          tx.object(vaultId),
          tx.object(poolId),
          coin,
          tx.pure.vector("u8", encryptedPointer),
        ],
      });

      await new Promise<void>((resolve, reject) => {
        signAndExecute(
          { transaction: tx },
          {
            onSuccess: async (result) => {
              addLog(`Deposit TX: ${result.digest}`);

              await new Promise((r) => setTimeout(r, 2000));

              const txResult = await suiClient.getTransactionBlock({
                digest: result.digest,
                options: { showEvents: true },
              });

              // Find TicketCreatedEvent
              const ticketEvent = txResult.events?.find((event: any) =>
                event.type?.includes("TicketCreatedEvent")
              );

              if (ticketEvent && ticketEvent.parsedJson) {
                const ticketId = ticketEvent.parsedJson.ticket_id;
                const tokenType = ticketEvent.parsedJson.token_type;

                setTickets((prev) => [
                  ...prev,
                  {
                    id: ticketId,
                    tokenType,
                    amount,
                    encryptedAmount: toHex(encryptedObject),
                    txUrl: `https://testnet.suivision.xyz/txblock/${result.digest}`,
                  },
                ]);

                addLog(`Ticket #${ticketId} created (${tokenType})`);

                // Reload tickets from vault
                await loadTicketsFromVault();

                resolve();
              } else {
                throw new Error("TicketCreatedEvent not found");
              }
            },
            onError: (error) => {
              addLog(`ERROR: Deposit failed: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`ERROR: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 2.5: TEST DECRYPTION (Security Verification)
  // ============================================================================

  const handleTestDecryption = async (ticketId: number) => {
    if (!sealClient || !vaultId || !account) {
      addLog("ERROR: Please ensure vault is created and wallet is connected");
      return;
    }

    try {
      setLoading(true);
      addLog(`Testing decryption for Ticket #${ticketId}...`);

      // Find the ticket
      const ticket = tickets.find((t) => t.id === ticketId);
      if (!ticket) {
        addLog("ERROR: Ticket not found");
        return;
      }

      // Step 1: Parse encrypted data
      addLog(`Parsing encrypted data...`);
      const encryptedBytes = fromHex(ticket.encryptedAmount);

      addLog(`Encrypted bytes length: ${encryptedBytes.length}`);

      const parsed = EncryptedObject.parse(new Uint8Array(encryptedBytes));

      // Ensure encryptionId is a proper Uint8Array
      let encryptionId: Uint8Array;
      if (parsed.id instanceof Uint8Array) {
        encryptionId = parsed.id;
      } else if (Array.isArray(parsed.id)) {
        encryptionId = new Uint8Array(parsed.id);
      } else if (typeof parsed.id === "string") {
        // It's a hex string, convert to Uint8Array
        encryptionId = fromHex(parsed.id);
      } else {
        throw new Error(`Unexpected encryption ID type: ${typeof parsed.id}`);
      }

      const encryptionIdHex = toHex(encryptionId);
      addLog(
        `🔑 Encryption ID (${
          encryptionId.length
        } bytes): ${encryptionIdHex.substring(0, 20)}...`
      );

      // Step 2: Build seal_approve_user PTB
      const tx = new Transaction();
      tx.moveCall({
        target: `${packageId}::seal_policy::seal_approve_user`,
        arguments: [
          tx.pure.vector("u8", Array.from(encryptionId)),
          tx.object(vaultId),
        ],
      });

      // Create fresh SuiClient for building tx
      const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

      // Build transaction bytes
      addLog(`Building transaction bytes...`);
      const txBytes = await tx.build({
        client: sealSuiClient,
        onlyTransactionKind: true,
      });

      // Step 3: Create session key and request signature
      addLog(`Creating session key...`);
      const sessionKey = await SessionKey.create({
        address: account.address,
        packageId,
        ttlMin: 10, // 10 minutes
        suiClient: sealSuiClient,
      });

      // Step 4: Request personal message signature from user
      addLog(`Requesting signature from wallet...`);
      const personalMessage = sessionKey.getPersonalMessage();

      await new Promise<void>((resolve, reject) => {
        signPersonalMessage(
          { message: personalMessage },
          {
            onSuccess: async (result) => {
              try {
                addLog(`Signature received`);

                // Set the signature on the session key
                await sessionKey.setPersonalMessageSignature(result.signature);

                // Step 5: Fetch decryption keys from SEAL servers
                addLog(`Fetching decryption keys...`);

                await sealClient.fetchKeys({
                  ids: [parsed.id],
                  txBytes,
                  sessionKey,
                  threshold: 2,
                });
                addLog(`Keys fetched successfully`);

                // Step 6: Decrypt locally
                addLog(`Decrypting locally...`);
                const decryptedData = await sealClient.decrypt({
                  data: encryptedBytes,
                  sessionKey,
                  txBytes,
                });

                // Step 7: Decode the decrypted amount
                const decoder = new TextDecoder();
                const decryptedAmount = decoder.decode(decryptedData);

                addLog(
                  `Decrypted amount: ${decryptedAmount} (${(
                    parseInt(decryptedAmount) / 1_000_000_000
                  ).toFixed(4)} SUI)`
                );
                addLog(`Decryption successful! You own this ticket.`);

                // Update ticket with decrypted amount
                setTickets((prev) =>
                  prev.map((t) =>
                    t.id === ticketId
                      ? { ...t, decryptedAmount, amount: decryptedAmount }
                      : t
                  )
                );

                resolve();
              } catch (error: any) {
                addLog(`ERROR: Decryption process failed: ${error.message}`);
                reject(error);
              }
            },
            onError: (error) => {
              addLog(`ERROR: Signature rejected: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`ERROR: Decryption failed: ${error.message || error}`);
      addLog(`   Details: ${JSON.stringify(error)}`);
      addLog(`   This is expected if you're not the vault owner!`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 3: SWAP - Query & Create Intents
  // ============================================================================

  const loadPendingIntents = async () => {
    if (!vaultId) {
      addLog("ERROR: Please select a vault first");
      return;
    }

    try {
      setLoading(true);
      addLog("Loading pending swap intents...");

      // Get IntentQueue object
      const queueObj = await suiClient.getObject({
        id: queueId,
        options: { showContent: true },
      });

      if (queueObj.data?.content?.dataType !== "moveObject") {
        addLog("ERROR: Invalid queue object");
        return;
      }

      const queueFields = queueObj.data.content.fields as any;
      const pendingTable = queueFields.pending;
      const pendingSize = parseInt(pendingTable.fields.size);

      addLog(`📋 Total pending intents in queue: ${pendingSize}`);

      if (pendingSize === 0) {
        addLog("📭 No pending intents");
        setPendingIntents([]);
        return;
      }

      // Get all pending intent IDs from the table
      const dynamicFields = await suiClient.getDynamicFields({
        parentId: pendingTable.fields.id.id,
      });

      addLog(`Querying ${dynamicFields.data.length} pending intents...`);

      // Load each intent object
      const intents = [];
      for (const field of dynamicFields.data) {
        try {
          const intentId = field.name.value as string;

          // Get the SwapIntent object
          const intentObj = await suiClient.getObject({
            id: intentId,
            options: { showContent: true },
          });

          if (intentObj.data?.content?.dataType === "moveObject") {
            const intentFields = intentObj.data.content.fields as any;

            // Check if this intent belongs to current vault
            if (intentFields.vault_id === vaultId) {
              const lockedTicketsSize = parseInt(
                intentFields.locked_tickets.fields.size
              );

              intents.push({
                id: intentId,
                vaultId: intentFields.vault_id,
                tokenOut: intentFields.token_out,
                minOutput: intentFields.min_output_amount,
                deadline: new Date(
                  parseInt(intentFields.deadline) * 1000
                ).toLocaleString(),
                user: intentFields.user,
                lockedTicketsCount: lockedTicketsSize,
                txUrl: `https://testnet.suivision.xyz/object/${intentId}`,
              });

              addLog(
                `  ✅ Intent: ${lockedTicketsSize} ticket(s) → ${intentFields.token_out}`
              );
            }
          }
        } catch (err: any) {
          addLog(`  WARNING: Failed to load intent: ${err.message}`);
        }
      }

      setPendingIntents(intents);
      addLog(`Found ${intents.length} pending intent(s) for this vault`);
    } catch (error: any) {
      addLog(`ERROR: Failed to load intents: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateSwapIntent = async () => {
    if (!vaultId || !account) {
      addLog("ERROR: Please select vault and connect wallet first");
      return;
    }

    if (selectedTicketIds.length === 0) {
      addLog("ERROR: Please select at least one ticket");
      return;
    }

    try {
      setLoading(true);
      addLog("Creating swap intent on-chain...");

      addLog(`   Tickets: [${selectedTicketIds.join(", ")}]`);
      addLog(`   Token Out: ${swapConfig.tokenOut}`);
      addLog(`   Slippage: ${swapConfig.slippagePercent}%`);

      // Calculate total input amount from selected tickets
      const totalInputAmount = selectedTicketIds.reduce((sum, ticketId) => {
        const ticket = tickets.find((t) => t.id === ticketId);
        return sum + (ticket ? parseInt(ticket.amount) : 0);
      }, 0);

      // Calculate minOutput based on slippage percentage
      // minOutput = totalInput * (1 - slippage/100)
      const slippage = parseFloat(swapConfig.slippagePercent);
      const minOutput = Math.floor(totalInputAmount * (1 - slippage / 100));

      addLog(`   Min Output: ${minOutput} (${slippage}% slippage protection)`);

      // Call create_swap_intent on-chain (no encryption needed!)
      addLog(`Submitting swap intent transaction...`);

      const tx = new Transaction();
      tx.moveCall({
        target: `${packageId}::mist_protocol::create_swap_intent`,
        arguments: [
          tx.object(queueId), // IntentQueue
          tx.object(vaultId), // VaultEntry
          tx.pure.vector("u64", selectedTicketIds),
          tx.pure.string(swapConfig.tokenOut),
          tx.pure.u64(minOutput),
          tx.pure.u64(swapConfig.deadline),
        ],
      });

      await new Promise<void>((resolve, reject) => {
        signAndExecute(
          { transaction: tx },
          {
            onSuccess: async (result) => {
              addLog(`Swap intent TX: ${result.digest}`);
              addLog(`SwapIntentEvent emitted on-chain!`);
              addLog(`Backend will detect and process the swap...`);
              addLog(
                `View TX: https://testnet.suivision.xyz/txblock/${result.digest}`
              );

              setSwapResult({
                digest: result.digest,
                txUrl: `https://testnet.suivision.xyz/txblock/${result.digest}`,
              });

              resolve();
            },
            onError: (error) => {
              addLog(`ERROR: Swap intent failed: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`ERROR: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // UI RENDERING
  // ============================================================================

  return (
    <div className="min-h-screen bg-black text-white">
      {/* Radial gradient background */}
      <div className="fixed inset-0 radial-gradient-bg pointer-events-none" />

      {/* Header */}
      <header className="relative border-b border-white/10 backdrop-blur-lg sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-8 py-4 flex justify-between items-center">
          <div className="flex items-center gap-4">
            <Image
              src="/assets/logo.svg"
              alt="Mist Protocol"
              width={48}
              height={48}
              className="opacity-90"
            />
            <div>
              <h1 className="text-3xl font-bold font-tektur gradient-text mb-1">
                Mist Protocol
              </h1>
            </div>
          </div>
          <ConnectButton />
        </div>
      </header>

      <div className="relative max-w-7xl mx-auto px-8 py-8">
        {/* Tab Navigation */}
        <div className="flex gap-3 mb-8">
          {[
            { key: "vault", label: "Vault" },
            { key: "wrap", label: "Wrap" },
            { key: "swap", label: "Swap" },
            { key: "unwrap", label: "Unwrap" },
          ].map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActivePhase(tab.key as any)}
              className={`glass-button px-6 py-2.5 font-medium font-tektur transition-all ${
                activePhase === tab.key
                  ? "border-white/20 text-white"
                  : "text-gray-400 hover:text-white hover:border-white/15"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Left Column - Actions */}
          <div className="space-y-6">
            {/* Vault Tab */}
            {activePhase === "vault" && (
              <div className="card p-6 border border-white/10">
                <h2 className="text-2xl font-bold mb-4 font-tektur">
                  Manage Your Vault
                </h2>
                <div className="space-y-4">
                  {/* Vault Discovery */}
                  <button
                    onClick={discoverVaults}
                    disabled={loading || !account}
                    className="w-full glass-button px-6 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {loading ? "Discovering..." : "Discover My Vaults"}
                  </button>

                  {/* Vault Selection */}
                  {availableVaults.length > 0 && (
                    <div>
                      <label className="block text-sm font-medium mb-2 font-anonymous-pro text-gray-400">
                        Select Vault ({availableVaults.length} available)
                      </label>
                      <select
                        value={vaultId}
                        onChange={async (e) => {
                          setVaultId(e.target.value);
                          await initializeSealClient();
                        }}
                        className="w-full bg-black/50 border border-white/10 px-4 py-2 text-white font-anonymous-pro backdrop-blur-sm"
                      >
                        {availableVaults.map((id, index) => (
                          <option key={id} value={id}>
                            Vault #{index + 1}: {id.substring(0, 20)}...
                          </option>
                        ))}
                      </select>
                    </div>
                  )}

                  {/* Manual Vault ID Input (fallback) */}
                  <div>
                    <label className="block text-sm font-medium mb-2 font-anonymous-pro text-gray-400">
                      Or Enter Vault ID Manually
                    </label>
                    <input
                      type="text"
                      value={vaultId}
                      onChange={(e) => setVaultId(e.target.value)}
                      placeholder="0x..."
                      className="w-full bg-black/50 border border-white/10 px-4 py-2 text-white font-mono backdrop-blur-sm"
                    />
                  </div>

                  {/* Create New Vault */}
                  <div className="border-t border-white/10 pt-4">
                    <p className="text-sm text-gray-400 mb-2 font-anonymous-pro">
                      Don&apos;t have a vault?
                    </p>
                    <button
                      onClick={handleCreateVault}
                      disabled={loading || !account}
                      className="w-full glass-button px-6 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {loading ? "Creating..." : "Create New Vault"}
                    </button>
                  </div>

                  <div className="text-sm text-gray-400 bg-black/30 border border-white/5 p-4 font-anonymous-pro">
                    <p className="font-medium mb-2 text-gray-300">Info:</p>
                    <ul className="list-disc list-inside space-y-1 text-gray-500">
                      <li>VaultRegistry tracks all your vaults (on-chain)</li>
                      <li>
                        VaultEntry stores encrypted tickets (shared object)
                      </li>
                      <li>SEAL client initialized for encryption/decryption</li>
                      <li>TEE can write output tickets to your vault</li>
                    </ul>
                  </div>
                </div>
              </div>
            )}

            {/* Wrap Tab */}
            {activePhase === "wrap" && (
              <div className="card p-6 border border-white/10">
                <h2 className="text-2xl font-bold mb-4 font-tektur">
                  Wrap Tokens
                </h2>
                <div className="space-y-4">
                  {/* Load Tickets Button */}
                  <button
                    onClick={loadTicketsFromVault}
                    disabled={loading || !vaultId}
                    className="w-full glass-button px-6 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {loading ? "Loading..." : "Load Tickets from Vault"}
                  </button>

                  {/* Deposit Buttons */}
                  <div className="border-t border-white/10 pt-4">
                    <p className="text-sm text-gray-400 mb-2 font-anonymous-pro">
                      Create new tickets:
                    </p>
                    <div className="grid grid-cols-2 gap-4">
                      <button
                        onClick={() => handleDepositSUI("100000000")}
                        disabled={loading || !vaultId}
                        className="glass-button px-4 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                      >
                        <img
                          src={TOKEN_ICONS.SUI}
                          alt="SUI"
                          className="w-4 h-4"
                        />
                        Deposit 0.1 SUI
                      </button>
                      <button
                        onClick={() => handleDepositSUI("500000000")}
                        disabled={loading || !vaultId}
                        className="glass-button px-4 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                      >
                        <img
                          src={TOKEN_ICONS.SUI}
                          alt="SUI"
                          className="w-4 h-4"
                        />
                        Deposit 0.5 SUI
                      </button>
                    </div>
                  </div>

                  <div className="text-sm text-gray-400 bg-black/30 border border-white/5 p-4 font-anonymous-pro">
                    <p className="font-medium mb-2">What happens:</p>
                    <ul className="list-disc list-inside space-y-1">
                      <li>Encrypts amount with SEAL (2-of-2 threshold)</li>
                      <li>Calls wrap_sui with vault + encrypted amount</li>
                      <li>Creates EncryptedTicket in vault</li>
                      <li>Emits TicketCreatedEvent</li>
                    </ul>
                    <p className="font-medium mt-3 mb-2 text-gray-300">
                      Security Testing:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-gray-500">
                      <li>
                        Click &quot;Test Decryption&quot; to verify you can
                        decrypt as owner
                      </li>
                      <li>
                        Only vault owner can decrypt via seal_approve_user
                      </li>
                      <li>Try from another wallet - it should fail!</li>
                    </ul>
                  </div>

                  {/* Tickets List */}
                  {tickets.length > 0 && (
                    <div className="bg-black/30 border border-white/5 p-4">
                      <h3 className="font-medium mb-3 font-tektur">
                        Your Tickets ({tickets.length})
                      </h3>
                      <div className="space-y-2">
                        {tickets.map((ticket) => (
                          <div
                            key={ticket.id}
                            className="bg-black/40 border border-white/10 p-3"
                          >
                            <div className="flex items-center justify-between mb-2">
                              <div className="flex items-center gap-2">
                                <img
                                  src={
                                    TOKEN_ICONS[
                                      ticket.tokenType as keyof typeof TOKEN_ICONS
                                    ] || TOKEN_ICONS.SUI
                                  }
                                  alt={ticket.tokenType}
                                  className="w-5 h-5"
                                />
                                <span className="font-medium">
                                  Ticket #{ticket.id}
                                </span>
                                <span className="text-gray-400 ml-2">
                                  {ticket.tokenType} -{" "}
                                  {(
                                    parseInt(ticket.amount) / 1_000_000_000
                                  ).toFixed(4)}{" "}
                                  tokens
                                </span>
                              </div>
                              <a
                                href={ticket.txUrl}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-blue-400 hover:text-blue-300 text-sm"
                              >
                                View TX →
                              </a>
                            </div>
                            <div className="flex gap-2">
                              <button
                                onClick={() => handleTestDecryption(ticket.id)}
                                disabled={loading}
                                className="flex-1 glass-button px-3 py-1.5 text-sm font-medium font-anonymous-pro disabled:opacity-50"
                              >
                                {ticket.decryptedAmount
                                  ? "Decrypted"
                                  : "Test Decryption"}
                              </button>
                              {ticket.decryptedAmount && (
                                <span className="text-gray-300 text-sm py-1.5 font-mono">
                                  {(
                                    parseInt(ticket.decryptedAmount) /
                                    1_000_000_000
                                  ).toFixed(4)}{" "}
                                  SUI
                                </span>
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Swap Tab */}
            {activePhase === "swap" && (
              <div className="card p-6 border border-white/10">
                <h2 className="text-2xl font-bold mb-4 font-tektur">
                  Private Swaps
                </h2>
                <div className="space-y-4">
                  {/* Load Pending Intents */}
                  <button
                    onClick={loadPendingIntents}
                    disabled={loading || !vaultId}
                    className="w-full glass-button px-6 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {loading ? "Loading..." : "Load Pending Swap Intents"}
                  </button>

                  {/* Show Pending Intents */}
                  {pendingIntents.length > 0 && (
                    <div className="bg-black/30 border border-white/5 p-4">
                      <h3 className="font-medium mb-3 font-tektur">
                        Pending Intents ({pendingIntents.length})
                      </h3>
                      <div className="space-y-2">
                        {pendingIntents.map((intent) => (
                          <div
                            key={intent.id}
                            className="bg-black/40 border border-white/10 p-3"
                          >
                            <div className="flex items-center justify-between mb-2">
                              <div className="flex items-center gap-2">
                                <span className="font-medium">
                                  {intent.lockedTicketsCount} ticket(s) locked
                                </span>
                                <span className="text-gray-400">→</span>
                                <img
                                  src={
                                    TOKEN_ICONS[
                                      intent.tokenOut as keyof typeof TOKEN_ICONS
                                    ] || TOKEN_ICONS.SUI
                                  }
                                  alt={intent.tokenOut}
                                  className="w-4 h-4"
                                />
                                <span className="text-gray-400">
                                  {intent.tokenOut}
                                </span>
                              </div>
                              <a
                                href={intent.txUrl}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-blue-400 hover:text-blue-300 text-sm"
                              >
                                View →
                              </a>
                            </div>
                            <div className="text-xs text-gray-400">
                              <p>Min Output: {intent.minOutput}</p>
                              <p>Deadline: {intent.deadline}</p>
                              <p className="text-yellow-400 mt-1">
                                Waiting for backend to process...
                              </p>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Ticket Selection */}
                  <div className="border-t border-white/10 pt-4">
                    <p className="text-sm text-gray-400 mb-2 font-anonymous-pro">
                      Create new swap intent:
                    </p>
                    <div>
                      <label className="block text-sm font-medium mb-2 font-anonymous-pro text-gray-400">
                        Select Tickets to Swap
                      </label>
                      <div className="space-y-2 bg-black/30 border border-white/5 p-3">
                        {tickets.length === 0 ? (
                          <p className="text-gray-400 text-sm">
                            No tickets available. Deposit first.
                          </p>
                        ) : (
                          tickets.map((ticket) => (
                            <label
                              key={ticket.id}
                              className="flex items-center gap-3 p-2 hover:bg-black/40 cursor-pointer border border-transparent hover:border-white/10 transition-colors"
                            >
                              <input
                                type="checkbox"
                                checked={selectedTicketIds.includes(ticket.id)}
                                onChange={(e) => {
                                  if (e.target.checked) {
                                    setSelectedTicketIds([
                                      ...selectedTicketIds,
                                      ticket.id,
                                    ]);
                                  } else {
                                    setSelectedTicketIds(
                                      selectedTicketIds.filter(
                                        (id) => id !== ticket.id
                                      )
                                    );
                                  }
                                }}
                                className="w-4 h-4"
                              />
                              <img
                                src={
                                  TOKEN_ICONS[
                                    ticket.tokenType as keyof typeof TOKEN_ICONS
                                  ] || TOKEN_ICONS.SUI
                                }
                                alt={ticket.tokenType}
                                className="w-4 h-4"
                              />
                              <span className="font-anonymous-pro">
                                Ticket #{ticket.id} - {ticket.tokenType} (
                                {(
                                  parseInt(ticket.amount) / 1_000_000_000
                                ).toFixed(4)}
                                )
                              </span>
                            </label>
                          ))
                        )}
                      </div>
                    </div>

                    {/* Swap Config */}
                    <div>
                      <label className="block text-sm font-medium mb-2 font-anonymous-pro text-gray-400">
                        Output Token
                      </label>
                      <div className="relative">
                        <img
                          src={
                            TOKEN_ICONS[
                              swapConfig.tokenOut as keyof typeof TOKEN_ICONS
                            ]
                          }
                          alt={swapConfig.tokenOut}
                          className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 pointer-events-none"
                        />
                        <select
                          value={swapConfig.tokenOut}
                          onChange={(e) =>
                            setSwapConfig({
                              ...swapConfig,
                              tokenOut: e.target.value,
                            })
                          }
                          className="w-full bg-black/50 border border-white/10 pl-11 pr-4 py-2 text-white font-anonymous-pro backdrop-blur-sm"
                        >
                          <option value="USDC">USDC</option>
                          <option value="SUI">SUI</option>
                        </select>
                      </div>
                    </div>

                    <div>
                      <label className="block text-sm font-medium mb-2 font-anonymous-pro text-gray-400">
                        Slippage Tolerance (%)
                      </label>
                      <div className="relative">
                        <input
                          type="number"
                          step="0.1"
                          min="0.1"
                          max="50"
                          value={swapConfig.slippagePercent}
                          onChange={(e) =>
                            setSwapConfig({
                              ...swapConfig,
                              slippagePercent: e.target.value,
                            })
                          }
                          className="w-full bg-black/50 border border-white/10 px-4 py-2 text-white font-mono backdrop-blur-sm"
                        />
                        <span className="absolute right-4 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none">
                          %
                        </span>
                      </div>
                      <p className="text-xs text-gray-500 mt-1">
                        Common values: 0.5% (low), 1% (medium), 3% (high)
                      </p>
                    </div>

                    <div className="text-sm text-gray-400 bg-black/30 border border-white/5 p-4 mb-4 font-anonymous-pro">
                      <p className="font-medium mb-2">What happens:</p>
                      <ul className="list-disc list-inside space-y-1">
                        <li>
                          Submits swap intent on-chain (ticket IDs, token out,
                          slippage)
                        </li>
                        <li>
                          Emits SwapIntentEvent with encrypted ticket amounts
                        </li>
                        <li>Backend event listener detects the swap request</li>
                        <li>TEE decrypts ticket amounts using SEAL</li>
                        <li>TEE executes swap on Cetus DEX</li>
                        <li>
                          TEE creates encrypted output tickets in your vault
                        </li>
                      </ul>
                      <p className="font-medium mt-3 mb-2">Privacy:</p>
                      <ul className="list-disc list-inside space-y-1">
                        <li>
                          Ticket amounts are SEAL encrypted (already done in
                          Phase 2)
                        </li>
                        <li>Only TEE can decrypt via seal_approve_tee</li>
                        <li>Swap amounts never revealed publicly</li>
                      </ul>
                    </div>
                  </div>

                  <button
                    onClick={handleCreateSwapIntent}
                    disabled={loading || selectedTicketIds.length === 0}
                    className="w-full glass-button px-6 py-3 font-medium font-anonymous-pro disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {loading ? "Creating..." : "Create Swap Intent"}
                  </button>

                  {swapResult && (
                    <div className="bg-white/5 border border-white/20 p-4">
                      <h3 className="font-medium text-white mb-2 font-tektur">
                        Swap Intent Created!
                      </h3>
                      <div className="text-sm space-y-1">
                        <p>Event emitted on-chain</p>
                        <a
                          href={swapResult.txUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-blue-400 hover:underline block"
                        >
                          View Transaction →
                        </a>
                        <p className="text-gray-400 mt-2">
                          Backend will process this swap automatically
                        </p>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Unwrap Tab */}
            {activePhase === "unwrap" && (
              <div className="card p-6 border border-white/10">
                <h2 className="text-2xl font-bold mb-4 font-tektur">
                  Unwrap Tokens
                </h2>
                <UnwrapCard />
              </div>
            )}
          </div>

          {/* Right Column - Logs & Status */}
          <div className="space-y-6">
            {/* Status Panel */}
            <div className="card p-6 border border-white/10">
              <h2 className="text-xl font-bold mb-4 font-tektur">Status</h2>
              <div className="space-y-3 text-sm font-anonymous-pro">
                <div>
                  <span className="text-gray-400">Wallet:</span>
                  <span className="ml-2 font-mono">
                    {account
                      ? `${account.address.substring(0, 10)}...`
                      : "Not connected"}
                  </span>
                </div>
                <div>
                  <span className="text-gray-400">Vault:</span>
                  <span className="ml-2 font-mono">
                    {vaultId ? `${vaultId.substring(0, 20)}...` : "Not created"}
                  </span>
                </div>
                <div>
                  <span className="text-gray-400">Tickets:</span>
                  <span className="ml-2">{tickets.length}</span>
                </div>
                <div>
                  <span className="text-gray-400">Selected:</span>
                  <span className="ml-2">{selectedTicketIds.length}</span>
                </div>
              </div>
            </div>

            {/* Logs Panel */}
            <div className="card p-6 border border-white/10">
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-bold font-tektur">Activity Logs</h2>
                <button
                  onClick={() => setLogs([])}
                  className="text-sm text-gray-400 hover:text-white font-anonymous-pro transition-colors"
                >
                  Clear
                </button>
              </div>
              <div className="bg-black/50 border border-white/5 p-4 h-[500px] overflow-y-auto font-mono text-xs space-y-1 backdrop-blur-sm">
                {logs.length === 0 ? (
                  <p className="text-gray-500">No activity yet...</p>
                ) : (
                  logs.map((log, i) => (
                    <div key={i} className="text-gray-300">
                      {log}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="relative border-t border-white/10 backdrop-blur-lg py-6 mt-8">
        <div className="max-w-7xl mx-auto px-8 flex justify-between items-center">
          <div className="text-sm text-gray-600 font-anonymous-pro">
            Powered by Nautilus • Seal • Walrus • Cetus
          </div>
          <div className="flex items-center gap-4">
            <a
              href="https://github.com/nikola0x0/mist-protocol"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-white transition-colors"
              aria-label="GitHub"
            >
              <svg
                className="w-5 h-5"
                fill="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
                  clipRule="evenodd"
                />
              </svg>
            </a>
            <a
              href="https://github.com/nikola0x0/mist-protocol/blob/main/README.md"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-white transition-colors"
              aria-label="Documentation"
            >
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
