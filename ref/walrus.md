# Walrus: Decentralized Blob Storage

## Overview

Walrus is a decentralized storage network built on Sui that provides efficient, cost-effective blob storage with strong availability guarantees. It enables storing and reading blobs while proving their availability using advanced erasure coding techniques.

## Key Concepts

### What is Walrus?

Walrus is a decentralized blob storage system that:
- Stores blobs ranging from small files to several GiB
- Provides ~5x storage overhead using advanced erasure coding (vs. >100x for traditional blockchain replication)
- Ensures content survives Byzantine faults in storage nodes
- Integrates with Sui blockchain for coordination and payments
- Supports Web2 HTTP access patterns through caches and CDNs

### Architecture

**Storage Nodes**
- Store shards containing slivers from all blobs
- Each blob is erasure-encoded into many slivers distributed across all storage nodes
- Assumes >2/3 of shards are managed by correct nodes within each storage epoch (2 weeks on Mainnet)
- Must tolerate up to 1/3 Byzantine storage nodes

**Clients and Users**
- Store and read blobs identified by blob IDs
- Pay for storage and non-best-effort reads
- Can prove blob availability to third parties without transmitting full blob

**Optional Infrastructure Actors** (permissionless):
- **Aggregators**: Reconstruct blobs from slivers and serve over HTTP
- **Caches**: Aggregators with caching to reduce latency and load
- **Publishers**: Help users store blobs over web2 protocols

**Sui Integration**
- All actors operate Sui blockchain clients
- Mediates payments, storage resources, shard mapping, and metadata
- Storage resources represented as Sui objects that can be acquired, owned, split, merged, and transferred

### Security Assumptions

Walrus provides security assuming:
- More than 2/3 of shards are managed by correct storage nodes within each storage epoch
- Can tolerate up to 1/3 Byzantine storage nodes
- Aggregators, publishers, and end users are not trusted
- Honest end users using honest intermediaries maintain security properties

## Core Operations

### Storage (Write Path)

1. **Acquire Storage Resource**: User purchases storage on Sui chain for appropriate size and duration
2. **Encode Blob**: Client erasure codes blob and computes blob ID
3. **Register Blob ID**: User updates storage resource on-chain to register blob ID (emits event for storage nodes)
4. **Distribute Slivers**:
   - User sends blob metadata to all storage nodes
   - Each sliver sent to storage node managing corresponding shard
5. **Storage Node Validation**:
   - Receives sliver and validates against blob ID
   - Checks authorization to store blob
   - Signs statement confirming it holds the sliver
6. **Certificate Creation**:
   - User aggregates 2/3+ shard signatures into availability certificate
   - Submits certificate to Sui chain
7. **Point of Availability (PoA)**: Verified certificate triggers availability event on Sui
8. **Synchronization**: Storage nodes automatically sync missing shards after PoA

**Key Details**:
- Need 2/3 of shard signatures to create certificate
- Code rate below 1/3 enables reconstruction from only 1/3 of shards
- Can use publisher service to handle encoding and distribution

### Retrieval (Read Path)

1. **Get Metadata**: Reader obtains blob metadata from any storage node
2. **Authenticate Metadata**: Verify metadata using blob ID
3. **Request Slivers**: Send parallel requests to storage nodes for shards
4. **Wait for Threshold**: Need f+1 responses to reconstruct
5. **Authenticate Slivers**: Verify slivers using blob ID
6. **Reconstruct Blob**: Decode blob and validate consistency
7. **Optional Caching**: Caches store reconstructed blob for future requests

**Consistency Checks**:
- **Default**: Validates first 334 primary slivers contain correct data
- **Strict**: Re-encodes entire blob to verify correct encoding (stronger guarantee)

### Refresh Availability

- Extension conducted fully on-chain
- User provides storage resource to extend blob availability period
- Emits event that storage nodes receive to extend storage duration

### Inconsistency Detection

If blob was incorrectly encoded:
1. Correct storage node detects reconstruction failure
2. Computes inconsistency proof for blob ID
3. Sends proof to all storage nodes for signature
4. Aggregates signatures into inconsistency certificate
5. Submits certificate to Sui chain
6. Storage nodes delete sliver data upon inconsistent event
7. Future reads return `None` for inconsistent blobs

## Encoding and Data Security

### Erasure Coding

**RedStuff** - Bespoke construction based on Reed-Solomon codes:
- Splits blob into k symbols and encodes into n > k symbols
- Can reconstruct from any 1/3 of symbols
- Systematic encoding (some nodes hold original data for fast random access)
- Deterministic encoding with no encoder discretion
- Results in 4.5-5x size expansion (independent of shard/node count)

### Data Authentication

**Blob ID Computation**:
1. Hash sliver representation in each shard
2. Build Merkle tree from hashes
3. Merkle tree root becomes blob hash
4. Blob ID derived from blob hash (authenticates all shard data and metadata)

**Verification**:
- Storage nodes use blob ID to verify sliver data belongs to blob
- Successful check proves data is as intended by writer

### Consistency Guarantees

**Data Consistency Property**: Any correct client reading a blob will either read the specific value authenticated by writer OR return an error.

**After Strict Consistency Check**: Any correct client will always succeed and read the same data during blob lifetime.

## Sui Integration

### Storage Resource Life Cycle

1. **System Object**: Holds committee of storage nodes, total available space, price per KiB
2. **Storage Purchase**: Users pay into storage fund separated by epochs
3. **Storage Resources**: Can be split, merged, transferred
4. **Blob Assignment**: Assign blob ID to storage resource (emits event)
5. **Certificate Upload**: Submit availability certificate to trigger PoA event
6. **Storage Extension**: Add storage resource with longer expiry to extend blob
7. **Inconsistency Proof**: Upload certificate if blob incorrectly encoded

### Governance Operations

**Storage Epochs**:
- Each epoch represented by Walrus system object
- Contains storage committee and shard-to-node mapping
- Users buy storage amount for one or more epochs (max ~2 years ahead)

**Payment Distribution**:
- Storage nodes perform light audits of each other
- Suggest which nodes receive payment based on audit performance
- Payments allocated at end of each epoch

### Challenge Mechanism

Storage nodes challenge shards during epoch:
1. Determine list of available blobs from Sui events
2. Provide seed to challenged shard
3. Compute challenge sequence based on seed and blob content
4. Challenged node responds with shard contents
5. Challenger determines if challenge passed based on thresholds
6. Report result on-chain
7. Sequential nature with timeout ensures timely storage

## Properties and Guarantees

Given 2/3+ honest shards per epoch:

**After Point of Availability**:
- Any correct user read gets value V (blob contents or None)
- All correct users reading get same value V
- Blob stored by correct user will read as that blob
- Correct user can always store blob and reach PoA

**Assurance Properties**:
- Correct user's blob: storage nodes can always recover correct slivers
- Failed sliver recovery: can produce inconsistency proof
- Correctly stored blob: no inconsistency proof can exist
- Blob with inconsistency proof: reads return None
- No delete operation: blob available for full availability period after PoA

**Rule of Thumb**: Before PoA, client ensures availability. After PoA, Walrus system maintains availability.

## Use Cases

### Storage of Media for NFTs/dApps
Store images, sounds, videos, game assets accessible via HTTP caches for multimedia dApps.

### AI Related Use Cases
- Store training datasets with verified provenance
- Store models, weights, proofs of correct training
- Ensure availability of AI model outputs

### Blockchain History Archival
Lower-cost decentralized storage for:
- Checkpoint sequences with transaction/effects content
- Historic blockchain state snapshots
- Code and binaries

### L2 Availability Support
Certify blob availability for L2s requiring data availability attestation, including:
- Validity proofs
- Zero-knowledge proofs
- Fraud proofs

### Fully Decentralized Web
Host complete web experiences including all resources (HTML, CSS, JS, media) with fully decentralized front-end and back-end.

### Subscription Media Models
- Creators store encrypted media on Walrus
- Provide decryption keys only to subscribers/paying users
- Walrus provides storage layer (encryption/decryption off-system)

## Walrus Sites

### Overview
Walrus Sites enable hosting fully decentralized websites with all resources stored on Walrus.

**Components**:
- Site builder for publishing sites to Walrus
- Portal for accessing sites
- Sui smart contracts for site metadata and routing

**Features**:
- Install site builder and publish sites
- Set SuiNS names for sites
- Advanced configuration (headers, routing, metadata)
- CI/CD integration (GitHub workflows)
- Bring your own domain
- Site data authentication
- Content delivery through portals

### Publishing Workflow

1. Install site builder
2. Build site resources locally
3. Publish to Walrus (stores resources as blobs)
4. Register site metadata on Sui
5. Optional: Set SuiNS name for human-readable URL
6. Access via portal (e.g., `https://site-name.walrus.site`)

### Technical Details

**Portal Operation**:
- Resolves site name from SuiNS/Sui
- Fetches site resources from Walrus
- Serves content to browser
- Authenticates data integrity

**Advanced Features**:
- Custom headers and metadata
- Client-side routing configuration
- Redirecting objects to Walrus Sites
- Custom domain support with DNS configuration
- SEO considerations for custom portals

## Developer Resources

### Client Interfaces

**CLI**: Command-line interface for direct interaction
```bash
walrus store <file>
walrus read <blob-id>
```

**JSON API**: Programmatic access with JSON responses

**HTTP API**: Web2-compatible HTTP endpoints for storing/retrieving blobs

**SDKs**: Libraries for various programming languages

### Storage Costs

Approximately 5x overhead on stored data size due to erasure coding, significantly lower than 100x+ replication cost on traditional blockchains.

### Sui Structures

Objects include:
- Storage resources (can be split, merged, owned, transferred)
- Blob metadata and certificates
- System objects for storage epochs

### Quilt

Advanced feature for storing/accessing patches of larger quilts for efficient partial reads.

## Operator Guide

### Running an Aggregator/Publisher

**Aggregator**:
- Reconstructs blobs from slivers
- Serves over HTTP
- Optional caching for performance

**Publisher**:
- Accepts blobs over HTTP
- Runs store protocol on user's behalf
- Encodes, distributes, collects signatures
- Handles on-chain actions

**Authenticated Publisher**: Variant with authentication for access control

### Running a Storage Node

Requirements:
- Manage one or more shards
- Store slivers from all blobs in assigned shards
- Respond to read requests
- Participate in synchronization
- Perform challenge/response for attestation

**Commission and Governance**:
- Storage nodes participate in governance
- Commission payments based on audit performance

**Backup and Restore**: Procedures for data persistence across restarts

### Upload Relay

Functioning as intermediary for upload operations with custom logic and bandwidth optimization.

## Staking and Unstaking

Mechanisms for participating in network security and earning rewards through staking tokens.

## Non-Objectives

Walrus explicitly does not:
- Reimplement a geo-replicated CDN with sub-10ms latency (but ensures CDN compatibility)
- Reimplement smart contract platform with consensus/execution (relies on Sui)
- Provide distributed key management for encryption (but can provide storage layer for such systems)

## Future Development

Planned features include:
- Minimal governance for storage node changes between epochs
- Periodic payment support for continued storage
- Storage attestation based on challenges
- Light nodes storing partial blobs with rewards
- Additional encoding scheme details

## Resources

- Whitepaper for full technical details
- Developer documentation
- Example applications
- Troubleshooting guides
- Terms of service and privacy policy
