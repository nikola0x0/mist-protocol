"use client";

import { useState } from "react";
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

export default function MistTestPage() {
  const account = useCurrentAccount();
  const suiClient = useSuiClient();
  const { mutate: signAndExecute } = useSignAndExecuteTransaction();
  const { mutate: signPersonalMessage } = useSignPersonalMessage();

  // State
  const [vaultId, setVaultId] = useState("");
  const [vaultObjectUrl, setVaultObjectUrl] = useState("");
  const [enclaveId, setEnclaveId] = useState("");
  const [tickets, setTickets] = useState<Array<{
    id: number;
    tokenType: string;
    amount: string;
    explorerUrl: string;
  }>>([]);
  const [poolId] = useState(process.env.NEXT_PUBLIC_POOL_ID || "");
  const [depositAmount, setDepositAmount] = useState("100000000"); // 0.1 SUI
  const [selectedTicketIds, setSelectedTicketIds] = useState<number[]>([]);
  const [swapIntent, setSwapIntent] = useState({
    token_out: "USDC",
    amount: "100000000",
    min_output: "95000000",
    deadline: Math.floor(Date.now() / 1000) + 3600,
  });
  const [encryptedData, setEncryptedData] = useState<Uint8Array | null>(null);
  const [swapResult, setSwapResult] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);

  const packageId = process.env.NEXT_PUBLIC_PACKAGE_ID!;
  const backendUrl =
    process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3001";

  // Create SuiClient for SEAL
  const sealSuiClient = new SuiClient({ url: getFullnodeUrl("testnet") });

  // SEAL client
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

  // Helper: Extract pure hex address from type identifier
  const sanitizeHexId = (input: string): string => {
    if (!input) return input;
    if (input.includes("::")) {
      return input.split("::")[0];
    }
    return input;
  };

  // Step 1: Create Vault
  const handleCreateVault = async () => {
    if (!account || !packageId) {
      addLog("❌ Please connect wallet");
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

      const result = await new Promise<any>((resolve, reject) => {
        signAndExecute(
          {
            transaction: tx,
          },
          {
            onSuccess: (result: any) => {
              console.log("Transaction result:", result);
              resolve(result);
            },
            onError: (error) => {
              reject(error);
            },
          }
        );
      });

      // Get the full transaction details to access objectChanges
      addLog("   Fetching transaction details...");
      const txResult = await suiClient.waitForTransaction({
        digest: result.digest,
        options: {
          showObjectChanges: true,
          showEffects: true,
        },
      });

      console.log("Full transaction result:", txResult);
      console.log("Object changes:", txResult.objectChanges);

      const vaultObj = txResult.objectChanges?.find(
        (obj: any) =>
          obj.type === "created" &&
          obj.objectType?.includes("VaultEntry")
      );

      if (vaultObj) {
        const id = sanitizeHexId(vaultObj.objectId);
        setVaultId(id);
        setVaultObjectUrl(`https://testnet.suivision.xyz/object/${id}`);
        addLog(`✅ Vault created: ${id}`);
        addLog(`   Explorer: https://testnet.suivision.xyz/object/${id}`);
      } else {
        addLog("⚠️ Could not find vault in objectChanges");
        console.error("Available objects:", txResult.objectChanges);
        throw new Error("Could not find vault object");
      }
    } catch (error: any) {
      addLog(`❌ Vault creation failed: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // Step 2: Deposit SUI and get eSUI
  const handleDepositSUI = async () => {
    if (!account || !vaultId || !packageId) {
      addLog("❌ Please create vault first");
      return;
    }

    try {
      setLoading(true);
      addLog("💰 Depositing SUI...");

      // First encrypt the deposit amount with SEAL
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const cleanVaultId = sanitizeHexId(vaultId);
      const vaultBytes = fromHex(cleanVaultId);
      const combined = new Uint8Array(vaultBytes.length + nonce.length);
      combined.set(vaultBytes, 0);
      combined.set(nonce, vaultBytes.length);
      const encryptionId = toHex(combined);

      addLog(`   Amount: ${depositAmount} (raw units)`);
      addLog(`   Encrypting balance pointer...`);

      // Encrypt the balance as the pointer
      const { encryptedObject } = await sealClient.encrypt({
        threshold: 2,
        packageId,
        id: encryptionId,
        data: new TextEncoder().encode(depositAmount),
      });

      const encryptedPointer = Array.from(encryptedObject);
      addLog(`✅ Balance encrypted (${encryptedPointer.length} bytes)`);

      if (!poolId) {
        throw new Error("Pool ID not configured. Please check environment variables.");
      }

      addLog(`   Using Pool: ${poolId.substring(0, 20)}...`);

      // Build wrap_sui transaction with vault parameter
      const tx = new Transaction();
      const [coin] = tx.splitCoins(tx.gas, [parseInt(depositAmount)]);

      tx.moveCall({
        target: `${packageId}::mist_protocol::wrap_sui`,
        arguments: [
          tx.object(vaultId), // Vault object
          tx.object(poolId), // Pool object
          coin,
          tx.pure.vector("u8", encryptedPointer),
        ],
      });

      const depositResult = await new Promise<any>((resolve, reject) => {
        signAndExecute(
          {
            transaction: tx,
          },
          {
            onSuccess: (result: any) => {
              resolve(result);
            },
            onError: (error) => {
              reject(error);
            },
          }
        );
      });

      // Get the full transaction details to extract ticket ID from events
      addLog("   Fetching transaction details...");
      const txResult = await suiClient.waitForTransaction({
        digest: depositResult.digest,
        options: {
          showObjectChanges: true,
          showEffects: true,
          showEvents: true,
        },
      });

      addLog(`✅ Deposit successful!`);

      // Find TicketCreatedEvent to get ticket ID
      const ticketEvent = txResult.events?.find(
        (event: any) => event.type?.includes("TicketCreatedEvent")
      );

      if (ticketEvent && ticketEvent.parsedJson) {
        const ticketId = ticketEvent.parsedJson.ticket_id;
        const tokenType = ticketEvent.parsedJson.token_type;
        addLog(`   Ticket #${ticketId} created (${tokenType})`);

        // Add ticket to state
        setTickets((prev) => [
          ...prev,
          {
            id: ticketId,
            tokenType,
            amount: depositAmount,
            explorerUrl: `https://testnet.suivision.xyz/txblock/${depositResult.digest}`,
          },
        ]);
      }

      addLog(`   TX: https://testnet.suivision.xyz/txblock/${depositResult.digest}`);
    } catch (error: any) {
      addLog(`❌ Deposit failed: ${error.message || error}`);
      console.error("Deposit error:", error);
    } finally {
      setLoading(false);
    }
  };

  // Step 3: Encrypt Swap Intent with SEAL
  const handleEncryptIntent = async () => {
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

      // Generate encryption ID (vault namespace + nonce)
      const nonce = crypto.getRandomValues(new Uint8Array(5));
      const cleanVaultId = sanitizeHexId(vaultId);
      const vaultBytes = fromHex(cleanVaultId);
      const combined = new Uint8Array(vaultBytes.length + nonce.length);
      combined.set(vaultBytes, 0);
      combined.set(nonce, vaultBytes.length);
      const encryptionId = toHex(combined);

      addLog(`   Encryption ID: ${encryptionId.substring(0, 40)}...`);
      addLog(`   Using tickets: ${selectedTicketIds.join(", ")}`);

      // Encode swap intent as JSON with ticket IDs
      const intent = {
        ticket_ids: selectedTicketIds,
        token_out: swapIntent.token_out,
        amount: swapIntent.amount,
        min_output: swapIntent.min_output,
        deadline: swapIntent.deadline,
      };
      const intentJson = JSON.stringify(intent);
      addLog(`   Intent: Tickets [${selectedTicketIds.join(",")}] → ${swapIntent.token_out}`);

      // Encrypt with SEAL
      const { encryptedObject } = await sealClient.encrypt({
        threshold: 2,
        packageId,
        id: encryptionId,
        data: new TextEncoder().encode(intentJson),
      });

      // Verify
      const parsed = EncryptedObject.parse(encryptedObject);
      if (parsed.services.length === 0) {
        throw new Error("Encryption failed - no key servers");
      }

      setEncryptedData(encryptedObject);
      addLog(`✅ Intent encrypted successfully!`);
      addLog(`   Key servers: ${parsed.services.length}`);
      addLog(`   Size: ${encryptedObject.length} bytes`);
    } catch (error: any) {
      addLog(`❌ Encryption failed: ${error.message || error}`);
      console.error("Encryption error:", error);
    } finally {
      setLoading(false);
    }
  };

  // Step 3: Send to TEE Backend for Processing
  const handleSendToTEE = async () => {
    if (!encryptedData || !vaultId) {
      addLog("❌ Please encrypt intent first");
      return;
    }

    try {
      setLoading(true);
      addLog("📤 Sending encrypted intent to TEE backend...");

      // Convert EncryptedObject to hex for backend (real SEAL decryption needs full EncryptedObject)
      const hexData = toHex(encryptedData);

      // Parse EncryptedObject to get encryption ID
      const parsed = EncryptedObject.parse(new Uint8Array(encryptedData));
      const encryptionIdHex = toHex(parsed.id);

      addLog(`   Encryption ID: ${encryptionIdHex.substring(0, 40)}...`);
      addLog(`   Data size: ${encryptedData.length} bytes`);

      const response = await fetch(`${backendUrl}/process_data`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          payload: {
            intent_id: `intent-${Date.now()}`,
            encrypted_data: hexData, // Full EncryptedObject as hex
            key_id: encryptionIdHex, // Encryption ID
            vault_id: vaultId,
            enclave_id: enclaveId || "0x0", // Use provided enclave or placeholder
          },
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Backend error: ${response.status} - ${errorText}`);
      }

      const result = await response.json();
      setSwapResult(result);

      addLog(`✅ TEE processed successfully!`);
      addLog(`   Executed: ${result.response?.data?.executed}`);
      if (result.response?.data) {
        const data = result.response.data;
        addLog(`   Input: ${data.input_amount} ${data.token_in}`);
        addLog(`   Output: ${data.output_amount} ${data.token_out}`);
        if (data.tx_hash) {
          addLog(`   TX: ${data.tx_hash.substring(0, 20)}...`);
        }
      }
    } catch (error: any) {
      addLog(`❌ TEE processing failed: ${error.message || error}`);
      console.error("TEE error:", error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-blue-900 to-gray-900 text-white">
      <div className="container mx-auto px-4 py-8">
        {/* Header */}
        <div className="flex justify-between items-center mb-8">
          <div>
            <h1 className="text-4xl font-bold">Mist Protocol Test</h1>
            <p className="text-gray-300 mt-2">
              Complete Flow: Vault → SEAL Encrypt → TEE Processing
            </p>
          </div>
          <ConnectButton />
        </div>

        {!account ? (
          <div className="text-center py-20">
            <p className="text-xl text-gray-300">Please connect your wallet to continue</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Left Column - Actions */}
            <div className="space-y-6">
              {/* Step 1: Create Vault */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
                  <span className="bg-blue-600 rounded-full w-8 h-8 flex items-center justify-center text-sm">
                    1
                  </span>
                  Create Vault
                </h2>
                <p className="text-gray-300 mb-4">
                  Create a vault to store encrypted ticket balances
                </p>
                <button
                  onClick={handleCreateVault}
                  disabled={loading || !!vaultId}
                  className="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold transition-colors"
                >
                  {vaultId ? "✅ Vault Created" : "Create Vault"}
                </button>
                {vaultId && (
                  <div className="mt-3 space-y-2">
                    <div className="p-3 bg-gray-900/50 rounded text-xs break-all">
                      <span className="text-gray-400">Vault ID:</span> {vaultId}
                    </div>
                    <a
                      href={vaultObjectUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="block text-center text-xs text-blue-400 hover:text-blue-300"
                    >
                      View on Explorer →
                    </a>
                  </div>
                )}
              </div>

              {/* Step 2: Deposit SUI */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
                  <span className="bg-blue-600 rounded-full w-8 h-8 flex items-center justify-center text-sm">
                    2
                  </span>
                  Deposit SUI (Create Ticket)
                </h2>
                <p className="text-gray-300 mb-4">
                  Deposit SUI to create an encrypted ticket in your vault
                </p>
                <div className="space-y-3">
                  <div>
                    <label className="block text-sm text-gray-300 mb-1">
                      Amount (in MIST, 1 SUI = 1,000,000,000 MIST)
                    </label>
                    <input
                      type="text"
                      value={depositAmount}
                      onChange={(e) => setDepositAmount(e.target.value)}
                      className="w-full px-3 py-2 bg-gray-900/50 border border-gray-600 rounded"
                      placeholder="100000000"
                    />
                    <p className="text-xs text-gray-500 mt-1">
                      Default: 100000000 = 0.1 SUI
                    </p>
                  </div>
                </div>
                <button
                  onClick={handleDepositSUI}
                  disabled={loading || !vaultId}
                  className="w-full mt-4 bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold transition-colors"
                >
                  Deposit SUI
                </button>
                {tickets.length > 0 && (
                  <div className="mt-3 p-3 bg-gray-900/50 rounded text-xs">
                    <span className="text-gray-400">Total Tickets:</span> {tickets.length}
                  </div>
                )}
              </div>

              {/* Step 3: Select Tickets & Configure Swap */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
                  <span className="bg-blue-600 rounded-full w-8 h-8 flex items-center justify-center text-sm">
                    3
                  </span>
                  Select Tickets & Configure Swap
                </h2>
                <div className="space-y-3">
                  {/* Ticket Selection */}
                  <div>
                    <label className="block text-sm text-gray-300 mb-2">
                      Select Tickets to Swap
                    </label>
                    {tickets.length === 0 ? (
                      <p className="text-sm text-gray-500">No tickets available. Deposit first.</p>
                    ) : (
                      <div className="space-y-2 max-h-40 overflow-y-auto">
                        {tickets.map((ticket) => (
                          <label
                            key={ticket.id}
                            className="flex items-center gap-2 p-2 bg-gray-900/50 rounded cursor-pointer hover:bg-gray-900"
                          >
                            <input
                              type="checkbox"
                              checked={selectedTicketIds.includes(ticket.id)}
                              onChange={(e) => {
                                if (e.target.checked) {
                                  setSelectedTicketIds([...selectedTicketIds, ticket.id]);
                                } else {
                                  setSelectedTicketIds(
                                    selectedTicketIds.filter((id) => id !== ticket.id)
                                  );
                                }
                              }}
                              className="w-4 h-4"
                            />
                            <span className="text-sm">
                              Ticket #{ticket.id} - {ticket.tokenType} ({ticket.amount} MIST)
                            </span>
                          </label>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Swap Configuration */}
                  <div>
                    <label className="block text-sm text-gray-300 mb-1">Token Out</label>
                    <select
                      value={swapIntent.token_out}
                      onChange={(e) =>
                        setSwapIntent({ ...swapIntent, token_out: e.target.value })
                      }
                      className="w-full px-3 py-2 bg-gray-900/50 border border-gray-600 rounded"
                    >
                      <option value="USDC">USDC</option>
                      <option value="SUI">SUI</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm text-gray-300 mb-1">Amount</label>
                    <input
                      type="text"
                      value={swapIntent.amount}
                      onChange={(e) =>
                        setSwapIntent({ ...swapIntent, amount: e.target.value })
                      }
                      className="w-full px-3 py-2 bg-gray-900/50 border border-gray-600 rounded"
                    />
                  </div>
                  <div>
                    <label className="block text-sm text-gray-300 mb-1">Min Output</label>
                    <input
                      type="text"
                      value={swapIntent.min_output}
                      onChange={(e) =>
                        setSwapIntent({ ...swapIntent, min_output: e.target.value })
                      }
                      className="w-full px-3 py-2 bg-gray-900/50 border border-gray-600 rounded"
                    />
                  </div>
                </div>
                <button
                  onClick={handleEncryptIntent}
                  disabled={loading || !vaultId || selectedTicketIds.length === 0 || !!encryptedData}
                  className="w-full mt-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold transition-colors"
                >
                  {encryptedData ? "✅ Intent Encrypted" : "Encrypt with SEAL"}
                </button>
              </div>

              {/* Step 4: Send to TEE */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
                  <span className="bg-blue-600 rounded-full w-8 h-8 flex items-center justify-center text-sm">
                    4
                  </span>
                  Send to TEE Backend
                </h2>
                <p className="text-gray-300 mb-4">
                  TEE will decrypt, execute swap (mock), and return result
                </p>
                <div className="mb-3">
                  <label className="block text-sm text-gray-300 mb-1">
                    Enclave ID (optional)
                  </label>
                  <input
                    type="text"
                    value={enclaveId}
                    onChange={(e) => setEnclaveId(e.target.value)}
                    placeholder="0x..."
                    className="w-full px-3 py-2 bg-gray-900/50 border border-gray-600 rounded text-sm"
                  />
                </div>
                <button
                  onClick={handleSendToTEE}
                  disabled={loading || !encryptedData}
                  className="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold transition-colors"
                >
                  Send to TEE & Execute
                </button>
              </div>
            </div>

            {/* Right Column - Logs & Results */}
            <div className="space-y-6">
              {/* Created Objects Summary */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-xl font-bold mb-4">Vault & Tickets</h2>
                <div className="space-y-3 text-sm">
                  {/* Vault */}
                  {vaultId ? (
                    <div className="p-3 bg-green-900/20 border border-green-700/30 rounded">
                      <div className="flex justify-between items-center mb-2">
                        <span className="text-green-400 font-semibold">✅ Vault</span>
                        <a
                          href={vaultObjectUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-xs text-blue-400 hover:underline"
                        >
                          Explorer →
                        </a>
                      </div>
                      <div className="text-xs text-gray-400 break-all">
                        {vaultId}
                      </div>
                    </div>
                  ) : (
                    <div className="p-3 bg-gray-900/50 border border-gray-700 rounded text-gray-500">
                      ⭕ Vault - Not created yet
                    </div>
                  )}

                  {/* Tickets */}
                  {tickets.length > 0 ? (
                    <div className="p-3 bg-green-900/20 border border-green-700/30 rounded">
                      <div className="flex justify-between items-center mb-2">
                        <span className="text-green-400 font-semibold">
                          ✅ Tickets ({tickets.length})
                        </span>
                      </div>
                      <div className="space-y-1 max-h-32 overflow-y-auto">
                        {tickets.map((ticket) => (
                          <div key={ticket.id} className="text-xs text-gray-400 flex justify-between">
                            <span>
                              #{ticket.id} - {ticket.tokenType}
                            </span>
                            <a
                              href={ticket.explorerUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="text-blue-400 hover:underline"
                            >
                              TX →
                            </a>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <div className="p-3 bg-gray-900/50 border border-gray-700 rounded text-gray-500">
                      ⭕ Tickets - No tickets yet
                    </div>
                  )}

                  {/* Encrypted Intent */}
                  {encryptedData ? (
                    <div className="p-3 bg-green-900/20 border border-green-700/30 rounded">
                      <span className="text-green-400 font-semibold">✅ Encrypted Intent</span>
                      <div className="text-xs text-gray-400 mt-1">
                        {encryptedData.length} bytes encrypted
                      </div>
                    </div>
                  ) : (
                    <div className="p-3 bg-gray-900/50 border border-gray-700 rounded text-gray-500">
                      ⭕ Encrypted Intent - Not created yet
                    </div>
                  )}

                  {/* Swap Result */}
                  {swapResult ? (
                    <div className="p-3 bg-green-900/20 border border-green-700/30 rounded">
                      <span className="text-green-400 font-semibold">✅ Swap Result</span>
                      <div className="text-xs text-gray-400 mt-1">
                        {swapResult.response?.data?.executed ? "Executed" : "Failed"}
                      </div>
                    </div>
                  ) : (
                    <div className="p-3 bg-gray-900/50 border border-gray-700 rounded text-gray-500">
                      ⭕ Swap Result - Not completed yet
                    </div>
                  )}
                </div>
              </div>

              {/* Logs */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-xl font-bold mb-4">Execution Log</h2>
                <div className="bg-gray-900/50 rounded p-4 h-[400px] overflow-y-auto font-mono text-sm">
                  {logs.length === 0 ? (
                    <p className="text-gray-500">No activity yet...</p>
                  ) : (
                    logs.map((log, i) => (
                      <div key={i} className="mb-1">
                        {log}
                      </div>
                    ))
                  )}
                </div>
                <button
                  onClick={() => setLogs([])}
                  className="mt-3 text-sm text-gray-400 hover:text-white"
                >
                  Clear Logs
                </button>
              </div>

              {/* Result */}
              {swapResult && (
                <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                  <h2 className="text-xl font-bold mb-4">Swap Result</h2>
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Status:</span>
                      <span className="text-green-400 font-semibold">
                        {swapResult.response?.data?.executed ? "✅ Executed" : "❌ Failed"}
                      </span>
                    </div>
                    {swapResult.response?.data && (
                      <>
                        <div className="flex justify-between">
                          <span className="text-gray-400">Input:</span>
                          <span>
                            {swapResult.response.data.input_amount}{" "}
                            {swapResult.response.data.token_in}
                          </span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-gray-400">Output:</span>
                          <span>
                            {swapResult.response.data.output_amount}{" "}
                            {swapResult.response.data.token_out}
                          </span>
                        </div>
                        {swapResult.response.data.tx_hash && (
                          <div className="mt-3 p-3 bg-gray-900/50 rounded text-xs break-all">
                            <span className="text-gray-400">TX Hash:</span>{" "}
                            {swapResult.response.data.tx_hash}
                          </div>
                        )}
                      </>
                    )}
                  </div>
                </div>
              )}

              {/* Flow Diagram */}
              <div className="bg-gray-800/50 rounded-lg p-6 border border-gray-700">
                <h2 className="text-xl font-bold mb-4">Flow</h2>
                <div className="space-y-2 text-sm">
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-blue-500"></div>
                    <span>User creates vault (on-chain)</span>
                  </div>
                  <div className="ml-3 border-l-2 border-gray-600 h-4"></div>
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-green-500"></div>
                    <span>Encrypt swap intent with SEAL</span>
                  </div>
                  <div className="ml-3 border-l-2 border-gray-600 h-4"></div>
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-purple-500"></div>
                    <span>TEE decrypts (SEAL threshold)</span>
                  </div>
                  <div className="ml-3 border-l-2 border-gray-600 h-4"></div>
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                    <span>TEE executes swap (mock/Cetus)</span>
                  </div>
                  <div className="ml-3 border-l-2 border-gray-600 h-4"></div>
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-red-500"></div>
                    <span>Return encrypted result</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
