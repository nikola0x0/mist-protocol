Here is a summary of the methods for implementing private transactions in blockchains, categorized by blockchain architecture and privacy goals.

# Summary: Methods of Private Transaction Implementation

This document systematizes the techniques used to achieve privacy in cryptocurrency transactions, distinguishing between **UTXO-based** and **Account-based** models, as well as general cross-cutting mechanisms.

## 1. General Privacy Techniques

These mechanisms can be applied across different architectures or implemented as overlay protocols.

### Stealth Addresses

- **Concept:** Allows a sender to generate a unique, one-time public key for a recipient for a single transaction, preventing observers from linking multiple payments to a single user.

- **Mechanism:** Uses an ephemeral key exchange (e.g., Elliptic Curve Diffie-Hellman) where the sender computes a shared secret to derive a destination address that only the receiver can spend.

- **Adoption:** Used in Monero, Bitcoin, and Ethereum.

### Mixing Techniques

Techniques to break the link between senders and receivers (unlinkability).

- **P2P Mixing (e.g., CoinJoin):** Users collaboratively combine inputs and outputs into a single large transaction to shuffle ownership without a central intermediary.

- **Centralized Tumblers (e.g., TumbleBit):** An untrusted intermediary redistributes funds. Techniques like blind signatures and cryptographic puzzles are used to prevent the tumbler from stealing funds or de-anonymizing users.

- **Smart Contract Mixers (e.g., Tornado Cash):** Users deposit funds into a pool managed by a smart contract and withdraw them later to a fresh address. ZK-proofs prove the user has a valid deposit without revealing which specific deposit is theirs.

---

## 2. Privacy in UTXO-based Blockchains

In this model, privacy is achieved by replacing explicit coins with cryptographic commitments.

### A. Confidentiality & Full Anonymity

These systems hide the sender, receiver, and amount, making a transaction indistinguishable from all other valid transactions (anonymity set = entire ledger).

- **Zerocoin & Zerocash (Zcash):**
- Uses **Mint** transactions to convert public coins into private coin commitments and **Pour** transactions to spend them.

- Relies on **zk-SNARKs** to prove a coin exists in the commitment tree without revealing its location.

- Requires a trusted setup for proof generation parameters.

- **Curve Trees (VCash):**
- Uses a trustless accumulator (Curve Trees) to achieve similar functionality to Zcash but without a trusted setup.

### B. Confidentiality & k-Anonymity

These systems hide the sender within a specific group of users (a "ring").

- **Ring Signatures (CryptoNote/Monero):** The sender creates a signature proving they are a member of a group of public keys without revealing which specific key signed the message.

- **RingCT (Ring Confidential Transactions):** Combines ring signatures with **Pedersen commitments** to hide transaction amounts while maintaining auditability of the money supply.

- **Lelantus (Firo):** Uses **one-out-of-many proofs** to hide the sender among a set of commitments and Bulletproofs to hide amounts, eliminating the need for a trusted setup.

- **Triptych:** An advanced ring signature scheme with logarithmic proof sizes, allowing for much larger anonymity sets () to improve privacy.

### C. Confidentiality & Unlinkability

- **Mimblewimble:** Combines **Pedersen commitments** (to hide values) with **CoinJoin** (to mix transactions) and **Cut-through** (to delete intermediate transaction states), achieving scalability and sender-receiver unlinkability.

- **Dash (PrivateSend):** Uses a masternode-coordinated CoinJoin implementation to mix funds in rounds.

---

## 3. Privacy in Account-based Blockchains

Achieving privacy here is complex because validating a transaction typically requires updating global state balances.

### A. Confidentiality Only

Hides transaction amounts but leaves sender/receiver relationships visible.

- **Homomorphic Encryption (Zether):** Balances are encrypted (e.g., using ElGamal). Validators homomorphically add/subtract encrypted transaction values to encrypted balances without seeing the actual amounts.

- **Twisted ElGamal (Solana):** Uses a variant of ElGamal encryption to support efficient zero-knowledge range and equality proofs.

- **Confidential ERC20 (Inco):** Uses Fully Homomorphic Encryption (FHE) to compare encrypted balances against transaction amounts validly.

### B. Confidentiality & k-Anonymity

Hides the sender among a set of dummy accounts included in the transaction.

- **Anonymous Zether:** Includes dummy accounts in a transaction. A ZK-proof ensures only the real sender/receiver balances change, while dummy balances are updated by zero (which is indistinguishable due to encryption).

- **Quisquis:** A hybrid model where users update key-value pairs (accounts). It uses updatable keys to prevent state bloat, though it suffers from front-running issues.

- **PriDe CT:** Allows batching multiple transactions to improve efficiency while providing receiver anonymity.

### C. Confidentiality & Full Anonymity

- **PriFHEte:** Uses **FHE** to obliviously update the entire state of account balances. This theoretically achieves full anonymity but requires validators to perform work (where is the total number of users), making it computationally expensive.

---

## 4. Regulatory Compliance Mechanisms

To balance privacy with regulation, specific "backdoors" or proving mechanisms are often integrated.

- **Viewing Keys (Auditability):** Allow a trusted auditor to see transaction details (amounts or identities) without making them public to the network.

- **ZK-Proofs for Policy (Accountability):** Users provide a zero-knowledge proof that a transaction satisfies specific rules (e.g., "amount < $10k" or "sender is not on a blocklist") without revealing the underlying data.
