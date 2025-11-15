# Nautilus + AWS Nitro Enclaves: Technical Deep Dive

**Date:** 2025-11-11
**Purpose:** Understanding how Nautilus uses AWS Nitro Enclaves for verifiable computation

---

## What is Nautilus?

Nautilus is a **framework** built by Mysten Labs for creating **verifiable off-chain computation** on Sui blockchain. It's not a standalone service—it's a template and pattern for building TEE (Trusted Execution Environment) applications.

**Key Concept:** Nautilus = Template + Patterns + Move Contracts for building TEE apps on AWS Nitro

---

## What is AWS Nitro Enclaves?

AWS Nitro Enclaves is a **hardware-based TEE** technology that provides:

### 1. **Isolated Computation Environment**
- Runs on special AWS EC2 instances (c6a, m6a, etc.)
- Complete isolation from parent EC2 instance
- No SSH access, no direct internet, no persistent storage
- Only VSOCK communication with parent instance

### 2. **Cryptographic Attestation**
- Generates **attestation documents** proving code is running in genuine TEE
- Signed by AWS as the root Certificate Authority
- Contains **PCR (Platform Configuration Register)** values

### 3. **Hardware Security Module (NSM - Nitro Secure Module)**
- Generates entropy (randomness)
- Signs attestation documents
- Provides secure key storage

---

## How Nautilus Uses AWS Nitro

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      USER / CLIENT                          │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 1. Request computation
                     │
┌────────────────────▼────────────────────────────────────────┐
│              AWS EC2 Instance (Parent)                      │
│  ┌────────────────────────────────────────────────────┐    │
│  │         AWS Nitro Enclave (Isolated TEE)           │    │
│  │                                                      │    │
│  │  ┌───────────────────────────────────────────┐     │    │
│  │  │  Nautilus Rust Server (Axum)             │     │    │
│  │  │  ┌─────────────────────────────────┐     │     │    │
│  │  │  │ Endpoints:                       │     │     │    │
│  │  │  │ - /health_check                  │     │     │    │
│  │  │  │ - /get_attestation               │     │     │    │
│  │  │  │ - /process_data (your logic)     │     │     │    │
│  │  │  └─────────────────────────────────┘     │     │    │
│  │  │                                            │     │    │
│  │  │  2. Generate ephemeral key pair          │     │    │
│  │  │  3. Call NSM API for attestation         │     │    │
│  │  │  4. Sign responses with ephemeral key    │     │    │
│  │  └───────────────────────────────────────────┘     │    │
│  │                                                      │    │
│  │  ┌──────────────────────────────────────────┐      │    │
│  │  │ NSM (Nitro Secure Module) - Hardware     │      │    │
│  │  │ - Generates attestation documents        │      │    │
│  │  │ - Provides entropy                        │      │    │
│  │  │ - Signs with AWS certificate chain       │      │    │
│  │  └──────────────────────────────────────────┘      │    │
│  └────────────────────────────────────────────────────┘    │
│                                                              │
│  Traffic Forwarder (Parent EC2)                             │
│  - Proxies API calls (enclave has no direct internet)       │
│  - Configured via allowed_endpoints.yaml                    │
└──────────────────────────────────────────────────────────────┘
                     │
                     │ 5. Submit signed response
                     │
┌────────────────────▼────────────────────────────────────────┐
│                  SUI BLOCKCHAIN                              │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │  EnclaveConfig (shared object)                      │    │
│  │  - PCR0, PCR1, PCR2 (expected values)             │    │
│  │  - Version                                          │    │
│  └────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Enclave (shared object)                            │    │
│  │  - Public key (from attestation)                   │    │
│  │  - Owner                                            │    │
│  └────────────────────────────────────────────────────┘    │
│                                                              │
│  6. Verify signature with registered public key            │
│  7. Execute smart contract logic                           │
└──────────────────────────────────────────────────────────────┘
```

---

## Key Components Breakdown

### 1. **PCR (Platform Configuration Registers)**

PCRs are **cryptographic hashes** that represent the enclave's state:

- **PCR0:** Hash of the enclave image file (EIF)
- **PCR1:** Hash of the Linux kernel running in enclave
- **PCR2:** Hash of the application code (your Rust server)

**Why PCRs matter:**
```bash
# Building the same code always produces the same PCRs
make ENCLAVE_APP=weather-example

# Output:
PCR0: 911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea...
PCR1: 911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea...
PCR2: 21b9efbc184807662e966d34f390821309eeac6802309798826296bf3e8bec7c...
```

**Reproducibility:** Anyone can rebuild your code and verify they get the same PCRs.

---

### 2. **Attestation Document**

Generated by NSM hardware inside the enclave:

```rust
// From common.rs (line 96-125)
pub async fn get_attestation(State(state): State<Arc<AppState>>)
    -> Result<Json<GetAttestationResponse>, EnclaveError> {

    let pk = state.eph_kp.public(); // Ephemeral key generated in enclave
    let fd = driver::nsm_init();    // Connect to NSM hardware

    // Request attestation with public key embedded
    let request = NsmRequest::Attestation {
        user_data: None,
        nonce: None,
        public_key: Some(ByteBuf::from(pk.as_bytes().to_vec())),
    };

    let response = driver::nsm_process_request(fd, request);
    // Returns attestation document signed by AWS
}
```

**Attestation document contains:**
- PCR values (proves what code is running)
- Public key (embedded by enclave)
- Certificate chain (signed by AWS)
- Timestamp

**Cannot be faked** because:
- Only real NSM hardware can generate valid attestations
- AWS certificate chain is verified on-chain
- PCRs are cryptographically bound to the code

---

### 3. **On-Chain Verification**

From `enclave.move` (lines 85-100):

```move
public fun register_enclave<T>(
    enclave_config: &EnclaveConfig<T>,
    document: NitroAttestationDocument,  // AWS attestation
    ctx: &mut TxContext,
) {
    let pk = enclave_config.load_pk(&document);  // Extract & verify

    let enclave = Enclave<T> {
        id: object::new(ctx),
        pk,  // Store verified public key
        config_version: enclave_config.version,
        owner: ctx.sender(),
    };

    transfer::share_object(enclave);  // Anyone can read this
}
```

**Verification happens in `load_pk` (implied):**
1. Check attestation signature (AWS certificate chain)
2. Verify PCR values match expected `EnclaveConfig`
3. Extract public key from attestation
4. Store public key on-chain

**After registration:** All future responses from enclave are verified by checking signature against stored public key (much cheaper than full attestation verification).

---

### 4. **Workflow: Registration → Usage**

#### **Step 1: Developer builds and deploys**

```bash
# 1. Build enclave locally (reproducible build)
make ENCLAVE_APP=weather-example

# Output: PCR values
PCR0=911c87d0...
PCR1=911c87d0...
PCR2=21b9efbc...

# 2. Publish Move contract with expected PCRs
sui client publish

# 3. Create enclave config on-chain
sui client call --function create_enclave_config \
  --args $CAP_OBJECT_ID "weather-enclave" \
         0x$PCR0 0x$PCR1 0x$PCR2

# 4. Deploy to AWS EC2 Nitro instance
make run
sh expose_enclave.sh

# 5. Get attestation from running enclave
curl http://<PUBLIC_IP>:3000/get_attestation
# Returns: {"attestation": "a3012663... (hex)"}

# 6. Register enclave on-chain with attestation
sui client call --function register_enclave \
  --args $ENCLAVE_CONFIG_ID <attestation_hex>
```

#### **Step 2: User requests computation**

```bash
# User sends request to enclave
curl -X POST http://<PUBLIC_IP>:3000/process_data \
  -d '{"payload": {"location": "San Francisco"}}'

# Enclave responds with signed data
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1744041600000,
    "data": {
      "location": "San Francisco",
      "temperature": 13
    }
  },
  "signature": "b75d2d44c4a6b3c676fe087465c0e85206b101e21be6cda4..."
}
```

#### **Step 3: On-chain verification**

```move
// User submits signed response to smart contract
public fun process_weather_data<T>(
    enclave: &Enclave<T>,
    response: vector<u8>,
    signature: vector<u8>,
) {
    // Verify signature using enclave's stored public key
    assert!(ed25519::verify(&signature, &response, &enclave.pk), 0);

    // Decode and use the verified data
    let weather_data: WeatherData = bcs::from_bytes(&response);

    // Execute application logic (e.g., mint NFT)
    mint_weather_nft(weather_data);
}
```

---

## Why AWS Nitro is Required

### **1. NSM Hardware Dependency**

From `lib.rs` (lines 28-55):

```rust
pub fn get_entropy(size: usize) -> Result<Vec<u8>, SystemError> {
    use nsm_lib::{nsm_get_random, nsm_lib_init};
    let nsm_fd = nsm_lib_init();  // Opens /dev/nsm device
    if nsm_fd < 0 {
        return Err(SystemError {
            message: String::from("Failed to connect to NSM device"),
        });
    };
    // ... uses hardware RNG
}
```

This code **only works inside Nitro Enclaves** because:
- `/dev/nsm` device doesn't exist outside enclaves
- Cannot be mocked without breaking security

### **2. Enclave Image Format (EIF)**

From `Containerfile`:
```dockerfile
FROM stagex/user-eif_build@sha256:... AS user-eif_build
FROM stagex/user-linux-nitro@sha256:... AS user-linux-nitro
```

The build process creates an **EIF binary** that:
- Only runs on Nitro hypervisor
- Contains embedded Linux kernel
- Has specific boot sequence for NSM initialization

### **3. Certificate Chain**

Attestation documents are signed with **AWS certificate chain**:
```
Root CA (AWS)
  └─> Region CA
      └─> Instance CA
          └─> Attestation Document
```

This chain **cannot be replicated** locally because:
- Private keys are in AWS hardware security modules
- On-chain verification checks AWS's root certificate

---

## What You Can Mock (For Hackathon)

While you can't mock the TEE itself, you can create a **demonstration backend**:

### Mock Backend Structure

```rust
// mock-backend/src/main.rs
use axum::{Json, Router, routing::{get, post}};
use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/get_attestation", get(mock_attestation))
        .route("/process_data", post(process_data));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_check() -> Json<Value> {
    json!({
        "pk": "mock_public_key_hex",
        "endpoints_status": {"api.cetus.zone": true}
    })
}

async fn mock_attestation() -> Json<Value> {
    json!({
        "mock": true,
        "attestation": "MOCK_ATTESTATION_FOR_DEMO",
        "warning": "This is not a real AWS Nitro attestation"
    })
}

async fn process_data(Json(payload): Json<Value>) -> Json<Value> {
    // Your intent processing logic here
    let intent = payload["payload"].clone();

    json!({
        "response": {
            "intent": 0,
            "timestamp_ms": chrono::Utc::now().timestamp_millis(),
            "data": {
                "processed": intent,
                "result": "mock_computation_result"
            }
        },
        "signature": "mock_signature_hex",
        "warning": "Mock signature - not from real TEE"
    })
}
```

### What This Demonstrates

✅ **Architecture:** Shows integration pattern
✅ **API Design:** Same endpoints as real Nautilus
✅ **Workflow:** Complete flow from request → response
✅ **Understanding:** Proves you understand the concept

❌ **Security:** No real TEE guarantees
❌ **Attestation:** Cannot verify on-chain
❌ **Reproducibility:** No PCR values

---

## Trust Model Comparison

### **Real Nautilus + AWS Nitro**

```
Trust: AWS Hardware + Open Source Code
Verification: Anyone can rebuild and check PCRs
Attack Surface: Minimal (isolated enclave)
Cost: ~$0.17/hr EC2
```

### **Encifher (Competitor)**

```
Trust: Centralized service (black box)
Verification: None (closed source)
Attack Surface: Unknown
Cost: Unknown
```

### **Mock Backend**

```
Trust: None (demonstration only)
Verification: N/A
Attack Surface: Full (regular server)
Cost: $0
```

---

## Key Takeaways

1. **Nautilus is a framework**, not a service
   - Provides templates and patterns
   - You build your own enclave
   - You control the infrastructure

2. **AWS Nitro provides the TEE**
   - Hardware-based security
   - Cryptographic attestation
   - Certificate chain verification

3. **On-chain verification makes it trustless**
   - PCRs stored on-chain
   - Public key registered on-chain
   - Anyone can verify signatures

4. **Reproducible builds enable decentralization**
   - Same code → same PCRs
   - Anyone can rebuild and verify
   - No need to trust the developer

5. **Cannot run without AWS Nitro** (for production)
   - NSM hardware dependency
   - EIF binary format
   - AWS certificate chain

---

## Recommended Hackathon Strategy

### **Option A: Mock + Documentation**
- Build mock backend with same API
- Document how real Nautilus would work
- Show architecture understanding
- Explain advantages over Encifher

**Demo script:**
> "We've built a mock Nautilus backend for integration testing. In production, this runs inside AWS Nitro Enclaves with real attestation. The key advantage is **verifiable reproducible builds**—anyone can rebuild our enclave and verify the PCR values match what's registered on-chain. This is impossible with Encifher's closed-source black box."

### **Option B: Real AWS (Risky)**
- Set up AWS account Day 1
- Deploy real enclave Day 4
- Risk: time constraint, debugging difficulty
- Reward: actual TEE demonstration

---

## Resources

- **Nautilus GitHub:** https://github.com/MystenLabs/nautilus
- **AWS Nitro Docs:** https://docs.aws.amazon.com/enclaves/latest/user/
- **Sui Nautilus Docs:** https://docs.sui.io/concepts/cryptography/nautilus
- **Nautilus Discord:** #nautilus in Sui Discord

---

**Conclusion:** Nautilus requires AWS Nitro for production use. For hackathon, mock backend + strong documentation can effectively demonstrate the concept and architectural advantages.
