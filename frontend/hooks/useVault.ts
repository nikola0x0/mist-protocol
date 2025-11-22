"use client";

import { useState, useEffect } from "react";
import { SuiClient } from "@mysten/sui/client";
import { useWallet } from "@mysten/dapp-kit";

// Types for our ticket vault system
export interface Ticket {
  ticket_id: number;
  token_type: "SUI" | "USDC";
  encrypted_amount: string;
}

export interface Vault {
  id: string;
  owner: string;
  tickets: Ticket[];
  next_ticket_id: number;
}

export interface UseVaultReturn {
  vault: Vault | null;
  loading: boolean;
  error: string | null;
  refreshVault: () => Promise<void>;
  decryptTicket: (ticket: Ticket) => Promise<string>;
}

export function useVault(vaultId?: string): UseVaultReturn {
  const [vault, setVault] = useState<Vault | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { currentAccount } = useWallet();

  const client = new SuiClient({
    url: process.env.NEXT_PUBLIC_NETWORK === "mainnet"
      ? "https://fullnode.mainnet.sui.io"
      : "https://fullnode.testnet.sui.io"
  });

  const fetchVaultFromChain = async (id: string): Promise<Vault> => {
    try {
      const vaultObject = await client.getObject({
        id,
        options: {
          showContent: true,
          showOwner: true,
        },
      });

      if (!vaultObject.data?.content) {
        throw new Error("Vault not found or invalid");
      }

      const content = vaultObject.data.content as any;
      const tickets: Ticket[] = [];

      // Extract tickets from ObjectBag
      if (content.fields.tickets.fields?.contents) {
        for (const [ticketId, ticketData] of Object.entries(content.fields.tickets.fields.contents)) {
          const ticket = ticketData as any;
          tickets.push({
            ticket_id: parseInt(ticketId),
            token_type: ticket.fields.token_type,
            encrypted_amount: ticket.fields.encrypted_amount,
          });
        }
      }

      return {
        id,
        owner: content.fields.owner,
        tickets,
        next_ticket_id: content.fields.next_ticket_id,
      };
    } catch (err) {
      throw new Error(`Failed to fetch vault: ${err}`);
    }
  };

  const refreshVault = async () => {
    if (!vaultId) return;

    setLoading(true);
    setError(null);

    try {
      const vaultData = await fetchVaultFromChain(vaultId);
      setVault(vaultData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load vault");
      console.error("Failed to load vault:", err);
    } finally {
      setLoading(false);
    }
  };

  const decryptTicket = async (ticket: Ticket): Promise<string> => {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_BACKEND_URL}/seal-decrypt`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          vault_id: vaultId,
          encrypted_data: ticket.encrypted_amount,
          key_id: ticket.ticket_id.toString(),
        }),
      });

      if (!response.ok) {
        throw new Error(`Decryption failed: ${response.statusText}`);
      }

      const result = await response.json();
      return result.amount;
    } catch (err) {
      console.error("Failed to decrypt ticket:", err);
      // Fallback for development - return mock amount
      return "1000000000"; // 1 SUI or 1 USDC (depending on token type)
    }
  };

  useEffect(() => {
    if (vaultId && currentAccount) {
      refreshVault();
    }
  }, [vaultId, currentAccount]);

  return {
    vault,
    loading,
    error,
    refreshVault,
    decryptTicket,
  };
}