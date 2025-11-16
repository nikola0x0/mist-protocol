# SEAL Decryption: Axum Backend (Direct Wallet) Pattern

This document outlines the correct architecture for performing SEAL decryption from a backend service (e.g., an Axum server) and clarifies why it differs from a frontend (dApp) implementation.

## 🚀 Executive Summary

The **Session Key** pattern (seen in the TypeScript SDK) is a **frontend UX abstraction** designed to avoid repeated wallet popups for a user.

Your **Axum backend** does not need this. It should use the **Direct Wallet Pattern**, where the server acts as the authenticated entity, signs transactions directly with its own secure wallet, and calls the SEAL API.

This is the standard, simplest, and most secure method for a backend service.

---

## 1. 🖥️ The Direct Wallet Pattern (Axum Server)

This is the recommended pattern for your backend.

### Why it Works

Your Axum server is a trusted, automated service, not an end-user. It can securely hold its own private key. The SEAL on-chain policy only cares that the transaction signer is an authorized address, which your server's wallet will be.

### Implementation Steps

1.  **Securely Load Key:** In your Axum app, load your service wallet's private key from a secure source (e.g., environment variables, a vault).
2.  **Build PTB:** Use the `sui-sdk` (Rust crate) to construct a `ProgrammableTransaction` that calls your on-chain `seal_approve` function, passing in the required `id`.
3.  **Sign PTB:** Sign this transaction _directly_ using your service wallet's private key.
4.  **Call SEAL API:** Use an HTTP client (like `reqwest`) to make a `POST` request to the SEAL key server's `/v1/fetch_key` endpoint.
5.  **Send Proof:** The body of your HTTP request will contain the signed, Base64-encoded `tx_bytes` (your PTB) as the proof of authorization.
6.  **Receive Plaintext:** The SEAL network verifies the PTB and its signature, checks it against the on-chain policy, and returns the decrypted data in the HTTP response.

### Required Rust Crates

- `sui-sdk`: For building and signing the `ProgrammableTransaction`.
- `reqwest`: (Or any other HTTP client) For calling the SEAL key server's API.

---

## 2. 📱 The Session Key Pattern (Frontend dApp)

This is the complex flow you see in the TypeScript SDK.

### Why it Exists

This pattern solves a **User Experience (UX) problem**. It would be terrible if a user had to approve a wallet signature popup for _every single file_ they want to decrypt.

### Implementation Steps

1.  **Create Session Key:** The dApp creates a new, temporary keypair (`SessionKey`) in the browser.
2.  **Sign Personal Message:** The dApp asks the user to sign _one_ `personalMessage`. This is a single, one-time popup.
3.  **Authorize Session Key:** The user's signature is used to "bless" the `SessionKey` for a short time (e.g., 10 minutes).
4.  **Sign PTB (with Session Key):** For the next 10 minutes, the dApp _automatically_ uses the temporary `SessionKey` to sign all `seal_approve` transactions in the background, requiring no new popups.

---

## 3. ✅ Confirmation & Evidence

Our confidence in the "Direct Wallet Pattern" is confirmed by the SEAL documentation and source code.

- **SEAL Design Doc (`/v1/fetch_key`):** The documentation for the key server's API states it requires "a **valid PTB**... evaluated against the `seal_approve*` rules." It _separately_ mentions "User confirmation and sessions" as a UX feature for dApps to avoid "repeated user confirmations."

- **`valid_ptb.rs` (Key Server Source Code):** This file is the server's validation logic. It proves our theory because of what it **checks for** and what it **ignores**:

  - **✅ It CHECKS:**

    - That the PTB is not empty.
    - That _all_ commands are `MoveCall`s.
    - That all `MoveCall`s are to the _same package_.
    - That all functions are named `seal_approve*`.

  - **❌ It DOES NOT CHECK:**
    - It **does not** contain any logic for a `SessionKey`.
    - It **does not** check for a `personalMessage` signature.
    - It **does not** care _how_ the PTB was signed, only that the _final signer_ is authorized by the on-chain policy.

### Bottom Line

The SEAL key server only cares about receiving a valid, safely-formed PTB. The `SessionKey` is a client-side trick to get that PTB from a user without annoying them. Your Axum server can skip this trick and create the PTB directly.
