/**
 * PKCE (Proof Key for Code Exchange) utilities for OAuth 2.0
 *
 * PKCE prevents authorization code interception attacks by:
 * 1. Generating a random code_verifier
 * 2. Creating a code_challenge (SHA256 hash of verifier)
 * 3. Sending challenge during auth, verifier during token exchange
 */

/**
 * Generate a cryptographically random code verifier
 * Must be 43-128 characters, using unreserved URI characters
 */
export function generateCodeVerifier(): string {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return base64URLEncode(array);
}

/**
 * Generate code challenge from verifier using SHA-256
 */
export async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const hash = await crypto.subtle.digest('SHA-256', data);
  return base64URLEncode(new Uint8Array(hash));
}

/**
 * Generate a random state parameter for CSRF protection
 */
export function generateState(): string {
  const array = new Uint8Array(16);
  crypto.getRandomValues(array);
  return base64URLEncode(array);
}

/**
 * Base64 URL encode (RFC 4648)
 * Different from standard base64: uses - and _ instead of + and /, no padding
 */
function base64URLEncode(buffer: Uint8Array): string {
  const base64 = btoa(String.fromCharCode(...buffer));
  return base64
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

// Storage keys
const STORAGE_KEYS = {
  CODE_VERIFIER: 'x_oauth_code_verifier',
  STATE: 'x_oauth_state',
} as const;

/**
 * Store PKCE values in sessionStorage (cleared when browser closes)
 */
export function storePKCE(codeVerifier: string, state: string): void {
  sessionStorage.setItem(STORAGE_KEYS.CODE_VERIFIER, codeVerifier);
  sessionStorage.setItem(STORAGE_KEYS.STATE, state);
}

/**
 * Retrieve and clear PKCE values from storage
 */
export function retrievePKCE(): { codeVerifier: string | null; state: string | null } {
  const codeVerifier = sessionStorage.getItem(STORAGE_KEYS.CODE_VERIFIER);
  const state = sessionStorage.getItem(STORAGE_KEYS.STATE);
  return { codeVerifier, state };
}

/**
 * Clear PKCE values from storage
 */
export function clearPKCE(): void {
  sessionStorage.removeItem(STORAGE_KEYS.CODE_VERIFIER);
  sessionStorage.removeItem(STORAGE_KEYS.STATE);
}
