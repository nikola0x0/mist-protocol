/**
 * Privacy Relayer Client
 *
 * Functions to submit swap intents via the relayer instead of directly.
 * This breaks the on-chain link between your wallet and your swap intent.
 *
 * Usage:
 *   // Instead of signing a transaction with your wallet:
 *   await submitIntentViaRelayer(encryptedDetails, "SUI", "SUI");
 *
 * Privacy benefit:
 *   - Direct: Observer sees "Your wallet → create_swap_intent"
 *   - Relayer: Observer sees "Relayer wallet → create_swap_intent"
 */

// ============ TYPES ============

export interface RelayerSubmitRequest {
  encryptedDetails: string;
  tokenIn: string;
  tokenOut: string;
  deadline?: number;
}

export interface RelayerSubmitResponse {
  success: boolean;
  txDigest?: string;
  error?: string;
}

export interface RelayerStatusResponse {
  status: "ready" | "not_configured";
  relayerAddress?: string;
  network?: string;
}

// ============ API FUNCTIONS ============

/**
 * Check if the relayer is configured and ready
 */
export async function checkRelayerStatus(): Promise<RelayerStatusResponse> {
  try {
    const response = await fetch("/api/relay/submit-intent", {
      method: "GET",
    });

    if (!response.ok) {
      return { status: "not_configured" };
    }

    return await response.json();
  } catch (error) {
    console.error("Failed to check relayer status:", error);
    return { status: "not_configured" };
  }
}

/**
 * Submit a swap intent via the privacy relayer
 *
 * @param encryptedDetails - SEAL encrypted intent (base64 string)
 * @param tokenIn - Input token type (e.g., "SUI")
 * @param tokenOut - Output token type (e.g., "SUI")
 * @param deadline - Optional deadline in milliseconds
 * @returns Transaction digest if successful
 */
export async function submitIntentViaRelayer(
  encryptedDetails: string,
  tokenIn: string,
  tokenOut: string,
  deadline?: number
): Promise<RelayerSubmitResponse> {
  try {
    const response = await fetch("/api/relay/submit-intent", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        encryptedDetails,
        tokenIn,
        tokenOut,
        deadline,
      } as RelayerSubmitRequest),
    });

    const result: RelayerSubmitResponse = await response.json();

    if (!response.ok) {
      return {
        success: false,
        error: result.error || `HTTP ${response.status}`,
      };
    }

    return result;
  } catch (error) {
    console.error("Failed to submit intent via relayer:", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Network error",
    };
  }
}
