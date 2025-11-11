# Seal: Decentralized Key Management System on Sui

## Overview

Seal is a decentralized key management system (KMS) that uses Identity-Based Encryption (IBE) to protect data with access policies defined on Sui. It enables developers to create encrypted data whose access is controlled by on-chain Move code, without exposing the data itself to key servers or blockchain validators.

## Core Concepts

### What is Seal?

Seal provides:
- **Identity-Based Encryption**: Encrypt data using package IDs and custom identities
- **On-Chain Access Control**: Define who can decrypt using Move smart contracts
- **Decentralized Key Servers**: Multiple key servers for resilience and privacy
- **Threshold Encryption**: t-out-of-n configuration for security and availability
- **No Data Visibility**: Key servers never see the encrypted data

### Key Features

- Access policies written in Move and enforced on Sui
- Support for multiple key servers with threshold encryption
- Time-lock encryption, subscriptions, allowlists, and voting patterns
- On-chain and off-chain decryption support
- Client-side encryption by default
- Session-based key access to minimize user confirmations

## Architecture

### Identity-Based Encryption (IBE)

**IBE Scheme consists of**:

1. **Setup**: Generate master secret key (msk) and master public key (mpk)
2. **Derive(msk, id)**: Generate derived secret key (sk) for identity
3. **Encrypt(mpk, id, m)**: Encrypt message m for identity
4. **Decrypt(sk, c)**: Decrypt ciphertext c with derived secret key

**Correctness Property**: For any identity and message, decryption of encrypted data with the correct derived key returns the original message.

### Components

#### 1. Access Policies on Sui

- Move package at address `PkgId` controls identity subdomain `[PkgId]*`
- Package defines authorization logic through Move code
- `seal_approve*` functions determine who can access keys
- Policies are transparent and publicly auditable on-chain

**Identity Namespace**: Package ID acts as namespace prefix for all related identities.

**Example Time-Lock Encryption**:
```move
entry fun seal_approve(id: vector<u8>, c: &clock::Clock) {
    let mut prepared: BCS = bcs::new(id);
    let t = prepared.peel_u64();
    let leftovers = prepared.into_remainder_bytes();

    // Check time has passed and identity fully consumed
    assert!((leftovers.length() == 0) && (c.timestamp_ms() >= t), ENoAccess);
}
```

Identity format: `[PkgId][bcs::to_bytes(T)]` where T is unlock time.

#### 2. Off-Chain Key Servers

**Key Server Responsibilities**:
- Hold IBE master secret key
- Derive keys for specific identities
- Verify on-chain access policies before returning keys
- Return keys only if policy approves request

**Key Server APIs**:
- `/v1/service`: Returns service information
- `/v1/fetch_key`: Handles derived key requests

**Request Requirements**:
- Signed by user's address using `signPersonalMessage`
- Valid Programmable Transaction Block (PTB) for policy evaluation
- Ephemeral encryption key to encrypt response

### Decentralization Model

#### Multi-Server Threshold Encryption

**t-out-of-n Configuration**:
- **Privacy**: Secure as long as fewer than t servers compromised
- **Liveness**: Available as long as at least t servers operational

**User Choice**:
- Users select key servers based on trust assumptions
- Servers vary in security (enclaves, air-gapped, jurisdiction)
- No mandatory key server set

**Important**: Key server set is fixed after encryption; cannot change encrypted data to use different servers.

#### Future MPC Committees

Planned support for:
- Multi-party computation (MPC) committee as single key server
- t-out-of-n configuration within committee
- Dynamic membership over time
- Sui validators or other participant groups

## Security Model

### Trust Assumptions

Encrypted data security relies on:

1. **Key Server Integrity**:
   - Seal key servers not compromised
   - OR fewer than threshold compromised (for threshold encryption)
   - Includes both key servers and dependent Sui full nodes

2. **Correct Access Control Policy**:
   - Policy accurately configured
   - If upgradeable, owner can modify at any time
   - Changes are transparent and publicly visible

### User Confirmation and Sessions

**Session Keys**:
- User approves key access request in wallet
- Approval granted per package
- Authorizes session key for limited time
- dApp retrieves keys without repeated confirmations

**Flow**:
1. dApp requests session key
2. User confirms in wallet
3. Session key grants temporary access
4. Multiple decryptions without re-confirmation

## Access Control Patterns

### Pattern: Private Data

**Use Case**: Single owner controls encrypted content

**Implementation**:
- Store ciphertext as owned object
- Only current owner can decrypt
- Transfer ownership without exposing data

**Applications**: Personal key storage, private NFTs, user credentials

### Pattern: Allowlist

**Use Case**: Share with defined group of approved users

**Implementation**:
- Manage access by adding/removing members
- Changes apply to future decryptions
- No need to re-encrypt data

**Applications**: Subscriptions, partner data rooms, early-access drops, timed public access

**Example**:
```move
entry fun seal_approve(id: vector<u8>, allowlist: &Allowlist) {
    let user = tx_context::sender(ctx);
    assert!(allowlist.contains(user), ENoAccess);
}
```

### Pattern: Subscription

**Use Case**: Time-limited access to content/services

**Implementation**:
- Define service with price and duration
- Subscribe to get time-limited pass
- Decrypt content until pass expires
- No re-encryption needed

**Applications**: Premium media, data feeds, paid API access, AI model access

**Example**:
```move
entry fun seal_approve(id: vector<u8>, subscription: &Subscription, clock: &Clock) {
    let user = tx_context::sender(ctx);
    let expiry = subscription.get_expiry(user);
    assert!(clock.timestamp_ms() < expiry, ENoAccess);
}
```

### Pattern: Time-Lock Encryption

**Use Case**: Auto-unlock at specific time

**Implementation**:
- Encrypt once with unlock timestamp
- Before time: no one can decrypt
- After time: authorized parties can decrypt
- Optional: extend unlock time before expiry

**Applications**:
- Coordinated reveals (drops, auctions)
- MEV-resistant trading
- Secure voting
- Scheduled disclosures

**Variation: Pre-signed URLs**:
- Gate Walrus blob behind time-limited link
- Combine time check with access rule
- Limited-time downloads without re-encryption

### Pattern: Secure Voting

**Use Case**: Encrypted ballots until completion

**Implementation**:
- Define eligible voters
- Each submits encrypted vote
- After completion, fetch threshold keys
- Use on-chain decryption for verifiable tally
- Invalid/tampered ballots ignored

**Applications**: Governance, sealed-bid auctions, time-locked voting

## Using Seal

### Access Control Management

**Defining seal_approve* Functions**:

**Guidelines**:
1. Multiple functions possible per package (different logic/parameters)
2. First parameter must be requested identity (vector<u8>, without package prefix)
3. Abort if access not granted (no return value)
4. Use non-public entry functions for upgradability
5. Version shared objects for backward compatibility

**Testing**:
- Use Move tests locally
- Build and publish with Sui CLI

```bash
cd examples/move
sui move build
sui client publish
```

### Limitations

**Full Node Evaluation**:
- Functions evaluated via `dry_run_transaction_block` RPC
- Uses full node's local chain state view
- State may vary across nodes (asynchronous)

**Constraints**:
1. **State propagation delays**: Nodes may not reflect latest state
2. **Non-atomic evaluation**: Different nodes may see different states
3. **No ordering guarantees**: Don't rely on transaction order within checkpoint
4. **Side-effect free**: Cannot modify on-chain state
5. **Random module**: Output not secure/deterministic (avoid)
6. **No composition**: Only `seal_approve*` invoked directly

**Example of What to Avoid**:
```move
// BAD: Assumes specific counter ordering
entry fun seal_approve(id: vector<u8>, cnt1: &Counter, cnt2: &Counter) {
    assert!(cnt1.count == cnt2.count, ENoAccess);  // May fail due to interleaving
}
```

### Encryption

**Using Seal SDK**:

1. **Select Key Servers**:
   - Reference by KeyServer object ID
   - Fixed preconfigured set OR dynamic user selection
   - Verify URLs correspond to claimed servers (optional)

2. **Create Seal Client**:
```typescript
const suiClient = new SuiClient({ url: getFullnodeUrl('testnet') });

const serverObjectIds = [
  "0x73d05d62c18d9374e3ea529e8e0ed6161da1a141a94d3f76ae3fe4e99356db75",
  "0xf5d14a81a982144ae441cd7d64b09027f116a468bd36e7eca494f750591623c8"
];

const client = new SealClient({
  suiClient,
  serverConfigs: serverObjectIds.map((id) => ({
    objectId: id,
    weight: 1,  // Can contribute 1x towards threshold
  })),
  verifyKeyServers: false,  // Set true for verification
});
```

**Server Weighting**: Higher weight = more contributions toward threshold. Useful for trusted/reliable servers.

**Server Verification**: Enable to confirm URLs match claimed servers (adds latency).

3. **Encrypt Data**:
```typescript
const { encryptedObject: encryptedBytes, key: backupKey } = await client.encrypt({
    threshold: 2,
    packageId: fromHEX(packageId),
    id: fromHEX(id),
    data,
});
```

**Returns**:
- `encryptedObject`: BCS-serialized encrypted bytes (shareable)
- `key`: Symmetric key for backup/disaster recovery (keep secret)

**Note**: Encryption is randomized; same inputs produce different outputs each time.

**Tip**: Parse encrypted object: `EncryptedObject.parse(encryptedBytes)` returns metadata.

**Size Consideration**: Encryption doesn't conceal message size. Pad with zeros if size is sensitive.

**Advanced**: Encrypt ephemeral symmetric key, store key on Sui, content on Walrus. Update policies without modifying content.

### Decryption

**Step 1: Create Session Key**:
```typescript
const sessionKey = await SessionKey.create({
    address: suiAddress,
    packageId: fromHEX(packageId),
    ttlMin: 10,  // 10 minute TTL
    suiClient: new SuiClient({ url: getFullnodeUrl('testnet') }),
});

const message = sessionKey.getPersonalMessage();
const { signature } = await keypair.signPersonalMessage(message);
sessionKey.setPersonalMessageSignature(signature);
```

**Session Key Options**:
- Pass `Signer` in constructor (e.g., EnokiSigner)
- Set `mvr_name` for readable package name in wallet
- Store in IndexedDB instead of localStorage for cross-tab persistence

**Step 2: Decrypt**:
```typescript
// Create Transaction for seal_approve evaluation
const tx = new Transaction();
tx.moveCall({
    target: `${packageId}::${moduleName}::seal_approve`,
    arguments: [
        tx.pure.vector("u8", fromHEX(id)),
        // other arguments
   ]
});

const txBytes = tx.build({ client: suiClient, onlyTransactionKind: true });

const decryptedBytes = await client.decrypt({
    data: encryptedBytes,
    sessionKey,
    txBytes,
});
```

**Transaction Requirements**:
- Only call `seal_approve*` functions
- All calls to same package
- Evaluated as if user sent transaction
- `TxContext::sender()` returns session key signer

**Debugging**: Use `dryRunTransactionBlock` directly with transaction block.

**Performance**: SealClient caches keys; reuse same instance to reduce backend calls.

**Multiple Keys**:
```typescript
await client.fetchKeys({
    ids: [id1, id2],
    txBytes: txBytesWithTwoSealApproveCalls,
    sessionKey,
    threshold: 2,
});
```

Reduces requests to key servers; recommended when multiple keys needed.

**Troubleshooting**: `InvalidParameter` error may mean recently created on-chain object not yet indexed. Wait and retry.

### On-Chain Decryption

Seal supports on-chain HMAC-CTR decryption in Move:

**Use Cases**: Auctions, secure voting, verifiable workflows

**Package IDs**:
- Testnet: `0x927a54e9ae803f82ebf480136a9bcff45101ccbe28b13f433c89f5181069d682`
- Mainnet: `0xa212c4c6c7183b911d0be8768f4cb1df7a383025b5d0ba0c014009f0f30f5f8d`

**Steps**:

1. **App Initialization**:
   - Get public keys: `client.getPublicKeys`
   - Convert: `bf_hmac_encryption::new_public_key`
   - Store on-chain
   - Users verify correctness before encryption

2. **Verify Derived Keys**:
   - Fetch: `client.getDerivedKeys` (returns map of server ID to key)
   - Convert to `Element<G1>` or `Element<G2>`: `from_bytes`
   - Call `bf_hmac_encryption::verify_derived_keys`
   - Returns `vector<VerifiedDerivedKey>`

3. **Perform Decryption**:
   - Call `bf_hmac_encryption::decrypt`
   - Returns `Option<vector<u8>>` (None if fails)

**TypeScript SDK Example**:
```typescript
// 1. Parse encrypted object
const encryptedObject = EncryptedObject.parse(encryptedBytes);

// 2. Get derived keys
const derivedKeys = await client.getDerivedKeys({
  id: encryptedObject.id,
  txBytes,
  sessionKey,
  threshold: encryptedObject.threshold,
});

// 3. Get public keys (do once during init)
const publicKeys = await client.getPublicKeys(
  encryptedObject.services.map(([service, _]) => service)
);

// 4-7. Build transaction for on-chain decryption
// (See full example in documentation)
```

## Performance Optimization

### General Recommendations

1. **Reuse SealClient**: Caches keys and onchain objects
2. **Reuse SessionKey**: Avoid repeated user confirmations
3. **Disable verification**: Set `verifyKeyServers: false` unless needed
4. **Fully specify objects**: Pass complete object references in PTBs
5. **Avoid unnecessary retrievals**: Rely on SDK caching
6. **Batch decryption**: Use `fetchKeys()` for multiple keys

### Encryption Strategy

**Choose AES for speed**:
- AES-256-GCM significantly faster than HMAC-CTR
- Use HMAC-CTR only for on-chain decryption of small data

**Envelope Encryption for Large Data**:
1. Generate symmetric key
2. Encrypt data with AES
3. Encrypt symmetric key with Seal
4. Store ciphertext (Walrus), key reference on Sui

**Benefits**:
- Better performance for large files
- Hardware acceleration available
- Safer key rotation without re-encrypting data
- Recommended for sensitive data

## Security Best Practices

### 1. Choose Appropriate Threshold

- Balance fault tolerance with security
- Too high threshold = data loss risk if servers offline
- Consider data sensitivity and lifetime
- Account for future key server availability

**Poor Example**: 5-of-5 (no fault tolerance)
**Good Example**: 3-of-5 (tolerates 2 server failures)

### 2. Vet Key Server Providers

**Seal is Permissionless**: Anyone can run key server, but selection is trust decision.

**Recommendations**:
- Choose trusted organizations/parties
- Establish business/legal agreements
- Document availability obligations
- Define incident response procedures
- Ensure service continuity terms

**Questions to Ask**:
- Full node dependency (self-managed, third-party, public)
- Redundancy and failover plans
- Upgrade cadence
- SLA commitments

### 3. Use Layered Encryption

For critical/large/long-lived data:

**Envelope Encryption Pattern**:
1. Generate own symmetric key
2. Encrypt data with symmetric key
3. Use Seal to encrypt/manage symmetric key
4. Rotate Seal key servers without re-encrypting data

**Benefits**:
- Update key servers without touching data
- Re-encrypt small key instead of large data
- Essential for immutable storage (Walrus)

### 4. Secure Symmetric Key Handling

**Symmetric Key from encrypt() API**:
- Returned for backup/disaster recovery
- Store securely if keeping
- OR return to user (user manages security)
- Leakage grants unauthorized access

**Note**: Different from layered encryption key.

### 5. Understand Key Leak Risks

**Client-Side Decryption Default**:
- Apps/users retrieve decryption key locally
- Key leak (intentional or not) enables unauthorized decryption
- No on-chain audit trail of key delivery

**Mitigations**:
- Implement application-level audit logging
- Log key access attempts and decryption events
- Store logs in tamper-evident system (Walrus)
- Anchor logs to chain if needed
- Support transparency and compliance

## Key Server Operations

### Operating Modes

#### Open Mode

**Characteristics**:
- Accepts requests for any package
- Single master key serves all packages
- Ideal for testing or best-effort services
- No direct user liability

**Setup**:
1. Generate BLS master key pair:
```bash
cargo run --bin seal-cli genkey
```

2. Register on-chain:
```bash
sui client call --function create_and_transfer_v1 \
  --module key_server \
  --package <SEAL_PACKAGE_ID> \
  --args <NAME> https://<URL> 0 <MASTER_PUBKEY>
```

3. Start server:
```bash
CONFIG_PATH=config.yaml MASTER_KEY=<MASTER_KEY> cargo run --bin key-server
```

**Config Requirements**:
- Set network (Testnet, Mainnet, !Custom)
- Set mode: `!Open`
- Set `key_server_object_id`

#### Permissioned Mode

**Characteristics**:
- Restricts to allowlisted packages
- Dedicated master key per client
- Recommended for B2B deployments
- Client-specific key separation

**Setup**:
1. Generate master seed:
```bash
cargo run --bin seal-cli gen-seed
```

2. Create config (initially empty clients)

3. Start server (prints unassigned public keys)

4. For each client:
   - Client provides policy package IDs
   - Register on-chain with derived public key
   - Add config entry with derivation index
   - Restart server

**Client Config Example**:
```yaml
- name: "alice"
  client_master_key: !Derived
    derivation_index: 0
  key_server_object_id: "<KEY_SERVER_OBJECT_ID>"
  package_ids:
    - "<POLICY_PACKAGE_ID_1>"
    - "<POLICY_PACKAGE_ID_2>"
```

**Important**: Add first published package version; upgrades automatically recognized.

### Export and Import Keys

**Export Key**:
```bash
cargo run --bin seal-cli derive-key --seed <MASTER_SEED> --index 0
```

**Disable on Current Server**:
```yaml
- name: "bob"
  client_master_key: !Exported
    deprecated_derivation_index: 0
```

**Transfer Object**:
```bash
sui transfer --object-id <KEY_SERVER_OBJECT_ID> --to <NEW_OWNER>
```

**Import on New Server**:
```yaml
- name: "bob"
  client_master_key: !Imported
    env_var: "BOB_BLS_KEY"
  key_server_object_id: "<KEY_SERVER_OBJECT_ID>"
  package_ids:
    - "<POLICY_PACKAGE_ID>"
```

Run with imported key:
```bash
CONFIG_PATH=config.yaml BOB_BLS_KEY=<CLIENT_MASTER_KEY> MASTER_KEY=<MASTER_SEED> cargo run --bin key-server
```

### Infrastructure Requirements

**Key Server Characteristics**:
- Lightweight, stateless service
- Easy horizontal scaling
- No persistent storage needed
- Requires trusted Sui Full node access

**Key Management**:
- Master key in cloud KMS OR self-managed vault
- Secure storage for imported keys
- Hardware vault support

**Deployment Recommendations**:
- Place behind API gateway/reverse proxy
- HTTPS with SSL/TLS termination
- Rate limiting and abuse prevention
- API key/token authentication
- Usage tracking for commercial offerings

**Observability**:
- Prometheus metrics on port 9184: `curl http://0.0.0.0:9184`
- Grafana visualization support
- Health check: `curl http://0.0.0.0:2024/health`

### CORS Configuration

Required for browser requests:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Request-Id, Client-Sdk-Type, Client-Sdk-Version, <API-KEY-HEADER>
Access-Control-Expose-Headers: x-keyserver-version
```

Add API key header name if required.

## Seal CLI

Command-line tool for key generation, encryption, decryption, and inspection.

### Generate Key Pairs

```bash
cargo run --bin seal-cli genkey
# Outputs: Masterkey and Publickey
```

### Encrypt

**AES Encryption**:
```bash
cargo run --bin seal-cli encrypt-aes \
  --message <HEX_MESSAGE> \
  --package-id <PACKAGE_ID> \
  --id <IDENTITY> \
  --threshold 2 \
  <PUBKEY1> <PUBKEY2> <PUBKEY3> -- \
  <SERVER_OBJ_ID1> <SERVER_OBJ_ID2> <SERVER_OBJ_ID3>
```

**Service Provider Encryption**:
```bash
cargo run --bin seal-cli encrypt \
  --secrets <HEX1>,<HEX2> \
  --ids <ID1>,<ID2> \
  -p <PACKAGE_ID> \
  -t 2 \
  -k <SERVER_OBJ_ID1>,<SERVER_OBJ_ID2> \
  -n testnet
```

### Decrypt

**With Symmetric Key**:
```bash
cargo run --bin seal-cli symmetric-decrypt \
  --key <SYMMETRIC_KEY> \
  <ENCRYPTED_OBJECT_HEX>
```

**With Threshold User Keys**:
```bash
# 1. Extract user secret keys
cargo run --bin seal-cli extract \
  --package-id <PACKAGE_ID> \
  --id <IDENTITY> \
  --master-key <MASTER_KEY>

# 2. Decrypt with threshold keys
cargo run --bin seal-cli decrypt \
  <ENCRYPTED_OBJECT_HEX> \
  <USER_KEY1> <USER_KEY2> -- \
  <SERVER_OBJ_ID1> <SERVER_OBJ_ID2>
```

### Fetch Keys

```bash
cargo run --bin seal-cli fetch-keys \
  --request <ENCODED_REQUEST> \
  -k <SERVER_OBJ_ID1>,<SERVER_OBJ_ID2> \
  -t 2 \
  -n testnet
```

### Inspect Encrypted Object

```bash
cargo run --bin seal-cli parse <ENCRYPTED_OBJECT_HEX>
```

Outputs human-readable format with version, package ID, identity, services, threshold, ciphertext, and encrypted shares.

## Cryptographic Primitives

### Currently Supported

**KEM (Key Encapsulation Mechanism)**:
- Boneh-Franklin IBE with BLS12-381 curve

**DEM (Data Encapsulation Mechanism)**:
- AES-256-GCM (recommended for most use cases - faster)
- HMAC-based CTR mode (for on-chain decryption only)

### Future Support

- Post-quantum primitives planned

### Advanced Encryption

For streaming, hardware-assisted, or chunked decryption:
- Use Seal as KMS to protect scheme's secret key
- Keep keys out of application code
- Enable complex decryption patterns

## Integration Examples

### Full End-to-End Tests
Available in integration tests repository.

### Example Applications
- Allowlist-gated content access
- NFT-gated content access
- Time-locked reveals
- Subscription-based media
- Secure voting systems

### Common Integration Pattern

1. Define access policy in Move
2. Deploy package to Sui
3. Select and configure key servers
4. Encrypt data with Seal SDK
5. Store encrypted data (Walrus/Sui)
6. Users request access via session key
7. Decrypt with verified access
8. Use decrypted data in application

## Best Practices Summary

### For Developers

1. **Design Policies Carefully**: Consider state propagation and full node limitations
2. **Test Locally**: Use Move tests before deployment
3. **Version Objects**: Support secure upgrades
4. **Document Assumptions**: Make trust model explicit
5. **Monitor Key Servers**: Track availability and performance

### For Users

1. **Verify Key Servers**: Check URLs and reputations
2. **Understand Thresholds**: Know recovery implications
3. **Backup Keys**: Secure symmetric keys if provided
4. **Audit Access**: Review what packages have access
5. **Check Policies**: Understand access control logic

### For Operators

1. **Secure Master Keys**: Use KMS or hardware vaults
2. **High Availability**: Deploy with redundancy
3. **Rate Limiting**: Prevent abuse
4. **Monitoring**: Track metrics and health
5. **Documentation**: Provide clear terms of service

## Pricing and Terms

Refer to official Seal documentation for:
- Key server pricing models
- Service level agreements
- Terms of service
- Privacy policy
- Usage limits and quotas

## Resources

- Seal SDK documentation
- Move patterns repository
- Integration examples
- Key server implementation
- Community Discord
- GitHub issues and discussions
