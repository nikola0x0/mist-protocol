import { API_BASE_URL } from '../utils/constants';
import type {
  SponsorTxRequestBody,
  CreateSponsoredTransactionApiResponse,
  ExecuteSponsoredTransactionApiInput,
  ExecuteSponsoredTransactionApiResponse,
} from '../types';

/**
 * Create a sponsored transaction using Enoki via backend
 *
 * @param body - The sponsor request containing network, txBytes, sender, and optional allowedAddresses
 * @returns The sponsored transaction bytes and digest
 */
export async function createSponsoredTransaction(
  body: SponsorTxRequestBody
): Promise<CreateSponsoredTransactionApiResponse> {
  const response = await fetch(`${API_BASE_URL}/api/sponsor`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      network: body.network,
      txBytes: body.txBytes,
      sender: body.sender,
      allowedAddresses: body.allowedAddresses || [],
    }),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new Error(errorData.error || 'Failed to create sponsored transaction');
  }

  return await response.json();
}

/**
 * Execute a sponsored transaction using Enoki via backend
 *
 * @param body - The execute request containing digest and signature
 * @returns The final transaction digest
 */
export async function executeSponsoredTransaction(
  body: ExecuteSponsoredTransactionApiInput
): Promise<ExecuteSponsoredTransactionApiResponse> {
  const response = await fetch(`${API_BASE_URL}/api/execute`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new Error(errorData.error || 'Failed to execute sponsored transaction');
  }

  return await response.json();
}
