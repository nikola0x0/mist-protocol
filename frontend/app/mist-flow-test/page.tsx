"use client";

import { useState } from "react";
import { ConnectButton } from "@/components/ConnectButton";
import {
  useCurrentAccount,
  useSuiClient,
  useSignAndExecuteTransaction,
} from "@mysten/dapp-kit";
import { Transaction } from "@mysten/sui/transactions";
import { SealClient, EncryptedObject } from "@mysten/seal";
import { fromHex, toHex } from "@mysten/sui/utils";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";

// Utility to sanitize hex IDs
const sanitizeHexId = (id: string): string => {
  if (!id) return "";
  return id.startsWith("0x") ? id : `0x${id}`;
};

export default function MistFlowTestPage() {
  const account = useCurrentAccount();
  const suiClient = useSuiClient();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();

  // Configuration
  const packageId = process.env.NEXT_PUBLIC_PACKAGE_ID!;
  const poolId = process.env.NEXT_PUBLIC_POOL_ID!;
  const backendUrl = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

  // State
  const [vaultId, setVaultId] = useState("");
  const [availableVaults, setAvailableVaults] = useState<string[]>([]);
  const [registryId, setRegistryId] = useState("");
  const [enclaveId, setEnclaveId] = useState(""); // Optional for now
  const [sealClient, setSealClient] = useState<SealClient | null>(null);

  // Tickets state
  const [tickets, setTickets] = useState<Array<{
    id: number;
    tokenType: string;
    amount: string;
    encryptedAmount: string;
    txUrl: string;
    decryptedAmount?: string; // For testing decryption
  }>>([]);

  // Swap state
  const [selectedTicketIds, setSelectedTicketIds] = useState<number[]>([]);
  const [swapConfig, setSwapConfig] = useState({
    tokenOut: "USDC",
    minOutput: "95000000",
    deadline: Math.floor(Date.now() / 1000) + 3600,
  });
  const [encryptedSwapIntent, setEncryptedSwapIntent] = useState<Uint8Array | null>(null);
  const [swapResult, setSwapResult] = useState<any>(null);

  // UI state
  const [loading, setLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [activePhase, setActivePhase] = useState<"setup" | "deposit" | "swap" | "withdraw">("setup");

  const addLog = (message: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
  };

  // ============================================================================
  // PHASE 1: SETUP - Vault Discovery
  // ============================================================================

  const discoverVaults = async () => {
    if (!account) {
      addLog("❌ Please connect wallet first");
      return;
    }

    try {
      setLoading(true);
      addLog("🔍 Discovering your vaults...");

      // Query user's VaultRegistry object
      const registries = await suiClient.getOwnedObjects({
        owner: account.address,
        filter: {
          StructType: `${packageId}::seal_policy::VaultRegistry`,
        },
        options: { showContent: true },
      });

      if (registries.data.length === 0) {
        addLog("📭 No vaults found. Create your first vault!");
        setAvailableVaults([]);
        return;
      }

      // Get the registry object details
      const registry = registries.data[0];
      const registryObjectId = registry.data?.objectId;
      setRegistryId(registryObjectId || "");

      addLog(`📋 Found registry: ${registryObjectId?.substring(0, 20)}...`);

      // Extract vault IDs from registry
      const registryDetails = await suiClient.getObject({
        id: registryObjectId!,
        options: { showContent: true },
      });

      if (registryDetails.data?.content?.dataType === "moveObject") {
        const fields = registryDetails.data.content.fields as any;
        const vaultIds = fields.vault_ids || [];

        addLog(`✅ Found ${vaultIds.length} vault(s)`);
        setAvailableVaults(vaultIds);

        // Auto-select first vault
        if (vaultIds.length > 0) {
          setVaultId(vaultIds[0]);
          addLog(`🗄️ Selected vault: ${vaultIds[0].substring(0, 20)}...`);
        }
      }
    } catch (error: any) {
      addLog(`❌ Discovery error: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateVault = async () => {
    if (!account) {
      addLog("❌ Please connect wallet first");
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
          { transaction: tx },
          {
            onSuccess: async (result) => {
              addLog(`✅ Vault creation TX: ${result.digest}`);

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
                addLog(`🗄️ Vault created: ${id}`);

                // Initialize SEAL client
                // Create a fresh SuiClient for SEAL (required for proper initialization)
                const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

                const server1 = process.env.NEXT_PUBLIC_SEAL_SERVER_1 || "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75";
                const server2 = process.env.NEXT_PUBLIC_SEAL_SERVER_2 || "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8";

                addLog(`🔑 SEAL Server 1: ${server1.substring(0, 20)}...`);
                addLog(`🔑 SEAL Server 2: ${server2.substring(0, 20)}...`);

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
                addLog("🔐 SEAL client initialized");

                // Refresh vault discovery
                await discoverVaults();

                resolve();
              } else {
                throw new Error("Vault object not found in transaction");
              }
            },
            onError: (error) => {
              addLog(`❌ Vault creation failed: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`❌ Error: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 2: DEPOSIT - Load & Create Tickets
  // ============================================================================

  const loadTicketsFromVault = async () => {
    if (!vaultId) {
      addLog("❌ Please select a vault first");
      return;
    }

    try {
      setLoading(true);
      addLog("📦 Loading tickets from vault...");

      // Get the vault object
      const vaultObj = await suiClient.getObject({
        id: vaultId,
        options: { showContent: true },
      });

      if (vaultObj.data?.content?.dataType !== "moveObject") {
        addLog("❌ Invalid vault object");
        return;
      }

      const fields = vaultObj.data.content.fields as any;
      const nextTicketId = parseInt(fields.next_ticket_id);

      addLog(`🎫 Vault has ${nextTicketId} ticket(s)`);

      if (nextTicketId === 0) {
        addLog("📭 No tickets in vault yet");
        setTickets([]);
        return;
      }

      // Load each ticket from the ObjectBag
      const loadedTickets = [];
      for (let i = 0; i < nextTicketId; i++) {
        try {
          // Query dynamic field (ticket in ObjectBag)
          const ticketField = await suiClient.getDynamicFieldObject({
            parentId: fields.tickets.fields.id.id,
            name: {
              type: "u64",
              value: i.toString(),
            },
          });

          if (ticketField.data?.content?.dataType === "moveObject") {
            const ticketFields = ticketField.data.content.fields as any;
            const encryptedAmountBytes = ticketFields.value.fields.encrypted_amount;
            const encryptedAmountHex = toHex(new Uint8Array(encryptedAmountBytes));

            loadedTickets.push({
              id: i,
              tokenType: ticketFields.value.fields.token_type,
              amount: "?", // We don't know the amount until decryption
              encryptedAmount: encryptedAmountHex,
              txUrl: `https://testnet.suivision.xyz/object/${ticketField.data.objectId}`,
            });

            addLog(`  ✅ Loaded Ticket #${i} (${ticketFields.value.fields.token_type})`);
          }
        } catch (err) {
          addLog(`  ⚠️ Ticket #${i} not found (may have been consumed)`);
        }
      }

      setTickets(loadedTickets);
      addLog(`✅ Loaded ${loadedTickets.length} tickets`);
    } catch (error: any) {
      addLog(`❌ Failed to load tickets: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDepositSUI = async (amount: string) => {
    if (!sealClient || !vaultId) {
      addLog("❌ Please create vault first");
      return;
    }

    try {
      setLoading(true);
      addLog(`💰 Depositing ${amount} MIST (${(parseInt(amount) / 1_000_000_000).toFixed(4)} SUI)...`);

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
        addLog(`❌ SEAL encryption error: ${encryptError.message}`);
        addLog(`   Details: ${JSON.stringify(encryptError)}`);
        throw encryptError;
      }

      const encryptedPointer = Array.from(encryptedObject);
      addLog(`✅ Amount encrypted (${encryptedPointer.length} bytes)`);

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
              addLog(`✅ Deposit TX: ${result.digest}`);

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

                addLog(`🎫 Ticket #${ticketId} created (${tokenType})`);

                // Reload tickets from vault
                await loadTicketsFromVault();

                resolve();
              } else {
                throw new Error("TicketCreatedEvent not found");
              }
            },
            onError: (error) => {
              addLog(`❌ Deposit failed: ${error.message}`);
              reject(error);
            },
          }
        );
      });
    } catch (error: any) {
      addLog(`❌ Error: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 2.5: TEST DECRYPTION (Security Verification)
  // ============================================================================

  const handleTestDecryption = async (ticketId: number) => {
    if (!sealClient || !vaultId || !account) {
      addLog("❌ Please ensure vault is created and wallet is connected");
      return;
    }

    try {
      setLoading(true);
      addLog(`🔓 Testing decryption for Ticket #${ticketId}...`);

      // Find the ticket
      const ticket = tickets.find((t) => t.id === ticketId);
      if (!ticket) {
        addLog("❌ Ticket not found");
        return;
      }

      // Step 1: Parse encrypted data
      addLog(`📝 Parsing encrypted data...`);
      const encryptedBytes = fromHex(ticket.encryptedAmount);
      const parsed = EncryptedObject.parse(encryptedBytes);
      const encryptionIdHex = toHex(parsed.id);

      addLog(`🔑 Encryption ID: ${encryptionIdHex.substring(0, 20)}...`);

      // Step 2: Build seal_approve_user PTB
      const tx = new Transaction();
      tx.moveCall({
        target: `${packageId}::seal_policy::seal_approve_user`,
        arguments: [
          tx.pure.vector("u8", Array.from(parsed.id)),
          tx.object(vaultId),
        ],
      });

      // Create fresh SuiClient for building tx
      const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

      // Build transaction bytes
      addLog(`🔨 Building transaction bytes...`);
      const txBytes = await tx.build({
        client: sealSuiClient,
        onlyTransactionKind: true
      });

      // Step 3: Fetch decryption keys from SEAL servers
      addLog(`🔑 Fetching decryption keys...`);
      const sessionKey = await sealClient.getSessionKey(account.address);

      await sealClient.fetchKeys({
        ids: [parsed.id],
        txBytes,
        sessionKey,
        threshold: 2,
      });

      // Step 4: Decrypt locally
      addLog(`🔓 Decrypting locally...`);
      const decryptedData = await sealClient.decrypt({
        data: encryptedBytes,
        sessionKey,
        txBytes,
      });

      // Step 5: Decode the decrypted amount
      const decoder = new TextDecoder();
      const decryptedAmount = decoder.decode(decryptedData);

      addLog(`✅ Decrypted amount: ${decryptedAmount} (${(parseInt(decryptedAmount) / 1_000_000_000).toFixed(4)} SUI)`);
      addLog(`✅ Original amount: ${ticket.amount}`);

      // Verify they match
      if (decryptedAmount === ticket.amount) {
        addLog(`✅ ✓ Decryption verified! Amounts match.`);
      } else {
        addLog(`⚠️ Warning: Decrypted amount doesn't match original`);
      }

      // Update ticket with decrypted amount
      setTickets((prev) =>
        prev.map((t) =>
          t.id === ticketId
            ? { ...t, decryptedAmount }
            : t
        )
      );

    } catch (error: any) {
      addLog(`❌ Decryption failed: ${error.message || error}`);
      addLog(`   Details: ${JSON.stringify(error)}`);
      addLog(`   This is expected if you're not the vault owner!`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // PHASE 3: SWAP - Encrypt Intent & Send to TEE
  // ============================================================================

  const handleEncryptSwapIntent = async () => {
    if (!sealClient || !vaultId) {
      addLog("❌ Please create vault first");
      return;
    }

    if (selectedTicketIds.length === 0) {
      addLog("❌ Please select at least one ticket");
      return;
    }

    try {
      setLoading(true);
      addLog("🔐 Encrypting swap intent with SEAL...");

      // Generate encryption ID
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const cleanVaultId = sanitizeHexId(vaultId);
      const vaultBytes = fromHex(cleanVaultId);
      const combined = new Uint8Array(vaultBytes.length + nonce.length);
      combined.set(vaultBytes, 0);
      combined.set(nonce, vaultBytes.length);
      const encryptionId = toHex(combined);

      addLog(`   Tickets: ${selectedTicketIds.join(", ")}`);
      addLog(`   Target: ${swapConfig.tokenOut}`);

      // Calculate total amount from selected tickets
      const totalAmount = selectedTicketIds.reduce((sum, ticketId) => {
        const ticket = tickets.find((t) => t.id === ticketId);
        return sum + parseInt(ticket?.amount || "0");
      }, 0);

      // Build swap intent
      const intent = {
        ticket_ids: selectedTicketIds,
        token_out: swapConfig.tokenOut,
        amount: totalAmount,
        min_output: parseInt(swapConfig.minOutput),
        deadline: swapConfig.deadline,
      };

      addLog(`   Amount: ${totalAmount} units`);

      // Encrypt with SEAL
      const intentJson = JSON.stringify(intent);
      const { encryptedObject } = await sealClient.encrypt({
        threshold: 2,
        packageId,
        id: encryptionId,
        data: new TextEncoder().encode(intentJson),
      });

      setEncryptedSwapIntent(encryptedObject);
      addLog(`✅ Swap intent encrypted (${encryptedObject.length} bytes)`);
    } catch (error: any) {
      addLog(`❌ Encryption failed: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSendToTEE = async () => {
    if (!encryptedSwapIntent || !vaultId) {
      addLog("❌ Please encrypt intent first");
      return;
    }

    try {
      setLoading(true);
      addLog("📤 Sending encrypted intent to TEE backend...");

      // Convert to hex
      const hexData = toHex(encryptedSwapIntent);
      const parsed = EncryptedObject.parse(new Uint8Array(encryptedSwapIntent));
      const encryptionIdHex = toHex(parsed.id);

      addLog(`   Encryption ID: ${encryptionIdHex.substring(0, 40)}...`);
      addLog(`   Data size: ${encryptedSwapIntent.length} bytes`);

      const response = await fetch(`${backendUrl}/process_data`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          payload: {
            intent_id: `intent-${Date.now()}`,
            encrypted_data: hexData,
            key_id: encryptionIdHex,
            vault_id: vaultId,
            enclave_id: enclaveId || "0x0",
          },
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Backend error: ${response.status} - ${errorText}`);
      }

      const result = await response.json();
      setSwapResult(result);

      addLog("✅ TEE processed swap successfully!");
      addLog(`   Input: ${result.response.data.input_amount} ${result.response.data.token_in}`);
      addLog(`   Output: ${result.response.data.output_amount} ${result.response.data.token_out}`);

      if (result.response.data.tx_hash) {
        addLog(`   TX: ${result.response.data.tx_hash}`);
      }
    } catch (error: any) {
      addLog(`❌ TEE processing failed: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // ============================================================================
  // UI RENDERING
  // ============================================================================

  return (
    <div className="min-h-screen bg-gradient-to-b from-gray-900 to-black text-white p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex justify-between items-center mb-8">
          <div>
            <h1 className="text-4xl font-bold mb-2">🌫️ Mist Protocol - Full Flow Test</h1>
            <p className="text-gray-400">Complete privacy-preserving swap flow with SEAL encryption</p>
          </div>
          <ConnectButton />
        </div>

        {/* Phase Navigation */}
        <div className="flex gap-2 mb-8">
          {["setup", "deposit", "swap", "withdraw"].map((phase) => (
            <button
              key={phase}
              onClick={() => setActivePhase(phase as any)}
              className={`px-6 py-2 rounded-lg font-medium transition-all ${
                activePhase === phase
                  ? "bg-blue-600 text-white"
                  : "bg-gray-800 text-gray-400 hover:bg-gray-700"
              }`}
            >
              {phase.charAt(0).toUpperCase() + phase.slice(1)}
            </button>
          ))}
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Left Column - Actions */}
          <div className="space-y-6">
            {/* Phase 1: Setup */}
            {activePhase === "setup" && (
              <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4">🏗️ Phase 1: Setup</h2>
                <div className="space-y-4">
                  {/* Vault Discovery */}
                  <button
                    onClick={discoverVaults}
                    disabled={loading || !account}
                    className="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 px-6 py-3 rounded-lg font-medium transition-colors"
                  >
                    {loading ? "Discovering..." : "🔍 Discover My Vaults"}
                  </button>

                  {/* Vault Selection */}
                  {availableVaults.length > 0 && (
                    <div>
                      <label className="block text-sm font-medium mb-2">
                        Select Vault ({availableVaults.length} available)
                      </label>
                      <select
                        value={vaultId}
                        onChange={(e) => setVaultId(e.target.value)}
                        className="w-full bg-gray-900 border border-gray-700 rounded px-4 py-2"
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
                    <label className="block text-sm font-medium mb-2">
                      Or Enter Vault ID Manually
                    </label>
                    <input
                      type="text"
                      value={vaultId}
                      onChange={(e) => setVaultId(e.target.value)}
                      placeholder="0x..."
                      className="w-full bg-gray-900 border border-gray-700 rounded px-4 py-2"
                    />
                  </div>

                  {/* Create New Vault */}
                  <div className="border-t border-gray-700 pt-4">
                    <p className="text-sm text-gray-400 mb-2">Don't have a vault?</p>
                    <button
                      onClick={handleCreateVault}
                      disabled={loading || !account}
                      className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 px-6 py-3 rounded-lg font-medium transition-colors"
                    >
                      {loading ? "Creating..." : "➕ Create New Vault"}
                    </button>
                  </div>

                  <div className="text-sm text-gray-400 bg-gray-900 rounded p-4">
                    <p className="font-medium mb-2">ℹ️ What happens:</p>
                    <ul className="list-disc list-inside space-y-1">
                      <li>VaultRegistry tracks all your vaults (on-chain)</li>
                      <li>VaultEntry stores encrypted tickets (shared object)</li>
                      <li>SEAL client initialized for encryption/decryption</li>
                      <li>TEE can write output tickets to your vault</li>
                    </ul>
                  </div>
                </div>
              </div>
            )}

            {/* Phase 2: Deposit */}
            {activePhase === "deposit" && (
              <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4">💰 Phase 2: Deposit & Create Tickets</h2>
                <div className="space-y-4">
                  {/* Load Tickets Button */}
                  <button
                    onClick={loadTicketsFromVault}
                    disabled={loading || !vaultId}
                    className="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 px-6 py-3 rounded-lg font-medium"
                  >
                    {loading ? "Loading..." : "📦 Load Tickets from Vault"}
                  </button>

                  {/* Deposit Buttons */}
                  <div className="border-t border-gray-700 pt-4">
                    <p className="text-sm text-gray-400 mb-2">Create new tickets:</p>
                    <div className="grid grid-cols-2 gap-4">
                      <button
                        onClick={() => handleDepositSUI("100000000")}
                        disabled={loading || !vaultId}
                        className="bg-green-600 hover:bg-green-700 disabled:bg-gray-700 px-4 py-3 rounded-lg font-medium"
                      >
                        Deposit 0.1 SUI
                      </button>
                      <button
                        onClick={() => handleDepositSUI("500000000")}
                        disabled={loading || !vaultId}
                        className="bg-green-600 hover:bg-green-700 disabled:bg-gray-700 px-4 py-3 rounded-lg font-medium"
                      >
                        Deposit 0.5 SUI
                      </button>
                    </div>
                  </div>

                  <div className="text-sm text-gray-400 bg-gray-900 rounded p-4">
                    <p className="font-medium mb-2">ℹ️ What happens:</p>
                    <ul className="list-disc list-inside space-y-1">
                      <li>Encrypts amount with SEAL (2-of-2 threshold)</li>
                      <li>Calls wrap_sui with vault + encrypted amount</li>
                      <li>Creates EncryptedTicket in vault</li>
                      <li>Emits TicketCreatedEvent</li>
                    </ul>
                    <p className="font-medium mt-3 mb-2">🔐 Security Testing:</p>
                    <ul className="list-disc list-inside space-y-1">
                      <li>Click "Test Decryption" to verify you can decrypt as owner</li>
                      <li>Only vault owner can decrypt via seal_approve_user</li>
                      <li>Try from another wallet - it should fail!</li>
                    </ul>
                  </div>

                  {/* Tickets List */}
                  {tickets.length > 0 && (
                    <div className="bg-gray-900 rounded p-4">
                      <h3 className="font-medium mb-3">🎫 Your Tickets ({tickets.length})</h3>
                      <div className="space-y-2">
                        {tickets.map((ticket) => (
                          <div key={ticket.id} className="bg-gray-800 rounded p-3">
                            <div className="flex items-center justify-between mb-2">
                              <div>
                                <span className="font-medium">Ticket #{ticket.id}</span>
                                <span className="text-gray-400 ml-2">
                                  {ticket.tokenType} - {(parseInt(ticket.amount) / 1_000_000_000).toFixed(4)} tokens
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
                                className="flex-1 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 px-3 py-1.5 rounded text-sm font-medium"
                              >
                                {ticket.decryptedAmount ? "✅ Decrypted" : "🔓 Test Decryption"}
                              </button>
                              {ticket.decryptedAmount && (
                                <span className="text-green-400 text-sm py-1.5">
                                  ✓ {(parseInt(ticket.decryptedAmount) / 1_000_000_000).toFixed(4)} SUI
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

            {/* Phase 3: Swap */}
            {activePhase === "swap" && (
              <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4">🔄 Phase 3: Privacy-Preserving Swap</h2>
                <div className="space-y-4">
                  {/* Ticket Selection */}
                  <div>
                    <label className="block text-sm font-medium mb-2">Select Tickets to Swap</label>
                    <div className="space-y-2 bg-gray-900 rounded p-3">
                      {tickets.length === 0 ? (
                        <p className="text-gray-400 text-sm">No tickets available. Deposit first.</p>
                      ) : (
                        tickets.map((ticket) => (
                          <label key={ticket.id} className="flex items-center gap-3 p-2 hover:bg-gray-800 rounded cursor-pointer">
                            <input
                              type="checkbox"
                              checked={selectedTicketIds.includes(ticket.id)}
                              onChange={(e) => {
                                if (e.target.checked) {
                                  setSelectedTicketIds([...selectedTicketIds, ticket.id]);
                                } else {
                                  setSelectedTicketIds(selectedTicketIds.filter((id) => id !== ticket.id));
                                }
                              }}
                              className="w-4 h-4"
                            />
                            <span>Ticket #{ticket.id} - {ticket.tokenType} ({(parseInt(ticket.amount) / 1_000_000_000).toFixed(4)})</span>
                          </label>
                        ))
                      )}
                    </div>
                  </div>

                  {/* Swap Config */}
                  <div>
                    <label className="block text-sm font-medium mb-2">Output Token</label>
                    <select
                      value={swapConfig.tokenOut}
                      onChange={(e) => setSwapConfig({ ...swapConfig, tokenOut: e.target.value })}
                      className="w-full bg-gray-900 border border-gray-700 rounded px-4 py-2"
                    >
                      <option value="USDC">USDC</option>
                      <option value="SUI">SUI</option>
                    </select>
                  </div>

                  <div>
                    <label className="block text-sm font-medium mb-2">Min Output (slippage protection)</label>
                    <input
                      type="text"
                      value={swapConfig.minOutput}
                      onChange={(e) => setSwapConfig({ ...swapConfig, minOutput: e.target.value })}
                      className="w-full bg-gray-900 border border-gray-700 rounded px-4 py-2"
                    />
                  </div>

                  <button
                    onClick={handleEncryptSwapIntent}
                    disabled={loading || selectedTicketIds.length === 0}
                    className="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 px-6 py-3 rounded-lg font-medium"
                  >
                    {loading ? "Encrypting..." : "🔐 Encrypt Swap Intent"}
                  </button>

                  {encryptedSwapIntent && (
                    <button
                      onClick={handleSendToTEE}
                      disabled={loading}
                      className="w-full bg-orange-600 hover:bg-orange-700 disabled:bg-gray-700 px-6 py-3 rounded-lg font-medium"
                    >
                      {loading ? "Processing..." : "📤 Send to TEE Backend"}
                    </button>
                  )}

                  {swapResult && (
                    <div className="bg-green-900/20 border border-green-700 rounded p-4">
                      <h3 className="font-medium text-green-400 mb-2">✅ Swap Executed!</h3>
                      <div className="text-sm space-y-1">
                        <p>Input: {swapResult.response.data.input_amount} {swapResult.response.data.token_in}</p>
                        <p>Output: {swapResult.response.data.output_amount} {swapResult.response.data.token_out}</p>
                        {swapResult.response.data.tx_hash && (
                          <p className="text-blue-400">TX: {swapResult.response.data.tx_hash.substring(0, 20)}...</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Phase 4: Withdraw */}
            {activePhase === "withdraw" && (
              <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4">💸 Phase 4: Unwrap & Withdraw</h2>
                <div className="space-y-4">
                  <div className="text-sm text-gray-400 bg-gray-900 rounded p-4">
                    <p className="font-medium mb-2">ℹ️ Coming soon:</p>
                    <ul className="list-disc list-inside space-y-1">
                      <li>View output tickets created by TEE</li>
                      <li>Decrypt ticket amounts</li>
                      <li>Call unwrap_ticket to withdraw</li>
                      <li>Receive real tokens back</li>
                    </ul>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Right Column - Logs & Status */}
          <div className="space-y-6">
            {/* Status Panel */}
            <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
              <h2 className="text-xl font-bold mb-4">📊 Status</h2>
              <div className="space-y-3 text-sm">
                <div>
                  <span className="text-gray-400">Wallet:</span>
                  <span className="ml-2 font-mono">
                    {account ? `${account.address.substring(0, 10)}...` : "Not connected"}
                  </span>
                </div>
                <div>
                  <span className="text-gray-400">Vault:</span>
                  <span className="ml-2 font-mono">
                    {vaultId ? `${vaultId.substring(0, 20)}...` : "Not created"}
                  </span>
                </div>
                <div>
                  <span className="text-gray-400">SEAL Client:</span>
                  <span className={`ml-2 ${sealClient ? "text-green-400" : "text-gray-500"}`}>
                    {sealClient ? "✅ Ready" : "⚪ Not initialized"}
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
            <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-bold">📝 Activity Logs</h2>
                <button
                  onClick={() => setLogs([])}
                  className="text-sm text-gray-400 hover:text-white"
                >
                  Clear
                </button>
              </div>
              <div className="bg-black rounded p-4 h-[500px] overflow-y-auto font-mono text-xs space-y-1">
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
    </div>
  );
}
