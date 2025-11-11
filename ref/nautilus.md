# Nautilus: Secure and Verifiable Off-Chain Computation on Sui

## Overview

Nautilus is a framework for secure and verifiable off-chain computation on Sui. It enables developers to delegate sensitive or resource-intensive tasks to a self-managed Trusted Execution Environment (TEE) while maintaining trust through smart contract-based on-chain verification.

## Core Concepts

### What is Nautilus?

Nautilus supports hybrid decentralized applications (dApps) requiring:
- **Private data handling**: Process sensitive information in isolated environments
- **Complex computations**: Offload resource-intensive operations from blockchain
- **Web2 integration**: Connect to external systems and APIs securely
- **Cryptographic verifiability**: Prove computation integrity on-chain

The framework ensures computations are:
- Tamper-resistant
- Isolated from external interference
- Cryptographically verifiable on-chain

### Current TEE Support

**AWS Nitro Enclaves** (Initial Support):
- Mature and production-ready TEE platform
- Supports reproducible builds for verification
- AWS-signed attestations verifiable on-chain
- Additional TEE providers may be added in future

## Architecture

### Components

Nautilus applications consist of two main components:

#### 1. Off-Chain Server
Runs inside a TEE (AWS Nitro Enclaves) and:
- Handles user input processing
- Executes scheduled tasks
- Performs sensitive computations
- Generates cryptographic attestations

#### 2. On-Chain Smart Contract
Written in Move and:
- Verifies TEE attestations before execution
- Validates computation results
- Enforces access control
- Manages state transitions

### How It Works

**Deployment Flow**:
1. Developer deploys off-chain server to self-managed TEE (AWS Nitro Enclaves)
2. TEE generates cryptographic attestation proving execution environment integrity
3. Sui smart contracts verify attestation on-chain before accepting TEE output
4. TEE integrity is auditable and anchored by provider's root of trust

**Trust Model**:
- Attestation document from AWS Nitro Enclave includes certificate chain
- Verify on-chain using AWS as root certificate authority
- Confirms enclave runs unmodified software (validated by PCR values)
- Users independently verify computation aligns with published source code

### Reproducible Builds

**Purpose**: Shift trust from runtime to build time

**Benefits**:
- Anyone can build and compare binary with source code
- Software changes result in different PCR values
- Unauthorized modifications are detectable
- Transparent verification without requiring runtime trust

**Process**:
1. Developer publishes source code publicly
2. Anyone builds code locally
3. Compare generated PCRs with on-chain registered values
4. Matching PCRs prove binary matches source code

**Note**: Not applicable when source code is not public

## Implementation Guide

### Developer Workflow

#### Phase 1: Server Setup

1. **Create Nautilus off-chain server**:
   - Use provided reproducible build template
   - Implement custom computation logic in Rust
   - Define allowed external endpoints

2. **Publish server code**:
   - Push to public repository (e.g., GitHub)
   - Ensures transparency and verifiability

3. **Register Platform Configuration Registers (PCRs)**:
   - PCRs are measurements of trusted computing base
   - Use Sui smart contract to register on-chain

4. **Deploy to AWS Nitro Enclave**:
   - Build enclave image
   - Launch on EC2 instance
   - Configure networking and access

5. **Register deployed enclave**:
   - Use Sui smart contract with attestation document
   - Include ephemeral public key for signing responses

6. **Optional: Backend services**:
   - Route access through backend for load balancing
   - Implement rate limiting
   - Reduces trusted computing base

**Key Tip**: Verify attestation documents on-chain only during enclave registration (high gas costs). After registration, use enclave key for efficient message verification.

#### Phase 2: User/Client Workflow

1. **(Optional) Verify server code**:
   - Build locally
   - Confirm PCRs match on-chain records

2. **Send request to enclave**:
   - Receive signed response

3. **Submit response on-chain**:
   - Verify before executing application logic

### Repository Structure

```
/move
  /enclave          # Utility functions for enclave config and public key registration
  /weather-example  # Example on-chain logic using enclave functions
  /twitter-example  # Alternative example

/src
  /aws              # AWS boilerplate (no modification needed)
  /init             # AWS boilerplate (no modification needed)
  /system           # AWS boilerplate (no modification needed)
  /nautilus-server  # Server that runs inside enclave
    /src
      /apps
        /weather-example
          mod.rs    # Defines process_data endpoint - CUSTOMIZE THIS
          allowed_endpoints.yaml  # Lists accessible endpoints - CUSTOMIZE THIS
        /twitter-example
    run.sh          # Runs Rust server (do not modify)
    common.rs       # Common code for attestation (do not modify)
```

### Key Implementation Files

**Customize These**:
- `allowed_endpoints.yaml`: Define external API access permissions
- `mod.rs`: Define application-specific computation logic
- Custom app directory under `/move` for Move modules
- Custom app directory under `/src/nautilus-server/src/apps`

**Do Not Modify**:
- `run.sh`: Handles server startup
- `common.rs`: Manages attestation retrieval
- `main.rs`: Initializes key pair and HTTP server
- AWS boilerplate directories

## Running an Enclave

### Prerequisites

- AWS developer account
- AWS CLI installed
- Key pair for EC2 access

### Setup Steps

1. **Configure environment variables**:
```bash
export KEY_PAIR=<your-key-pair-name>
export AWS_ACCESS_KEY_ID=<your-access-key>
export AWS_SECRET_ACCESS_KEY=<your-secret-key>
export AWS_SESSION_TOKEN=<your-session-token>
```

2. **Run configuration script**:
```bash
sh configure_enclave.sh <APP>  # e.g., weather-example
```

3. **Handle secrets** (optional):
   - Use AWS Secrets Manager for API keys
   - Avoid including secrets in public code
   - Pass secrets as environment variables to enclave

4. **Connect to EC2 instance**:
```bash
ssh -i your-key.pem ec2-user@<PUBLIC_IP>
```

5. **Build and run enclave**:
```bash
cd nautilus/
make ENCLAVE_APP=<APP> && make run
sh expose_enclave.sh  # Expose port 3000
```

6. **Test endpoints**:
```bash
# Health check
curl -H 'Content-Type: application/json' -X GET http://<PUBLIC_IP>:3000/health_check

# Get attestation
curl -H 'Content-Type: application/json' -X GET http://<PUBLIC_IP>:3000/get_attestation

# Process data
curl -H 'Content-Type: application/json' -d '{"payload": {"location": "San Francisco"}}' -X POST http://<PUBLIC_IP>:3000/process_data
```

### Available Endpoints

1. **`/health_check`**:
   - Probes allowed domains inside enclave
   - Built into template (no modification needed)

2. **`/get_attestation`**:
   - Returns signed attestation document over enclave public key
   - Used during on-chain registration
   - Built into template (no modification needed)

3. **`/process_data`**:
   - Custom endpoint for application logic
   - Developer implements this
   - Fetches external data, processes, signs with enclave key

### Local Development

Test `process_data` endpoint locally without full enclave:

```bash
cd src/nautilus-server/
RUST_LOG=debug API_KEY=<your-api-key> cargo run --features=<APP> --bin nautilus-server
curl -H 'Content-Type: application/json' -d '{"payload": {...}}' -X POST http://localhost:3000/process_data
```

**Note**: `/get_attestation` only works inside enclave (requires NSM driver access)

### Reproducible Builds

Verify PCRs locally:

```bash
cd nautilus/
make ENCLAVE_APP=<APP>
cat out/nitro.pcrs

# Example output:
# PCR0=911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea83a26aad51593ade1e47ab6cced4653e
# PCR1=911c87d0abc8c9840a0810d57dfb718865f35dc42010a2d5b30e7840b03edeea83a26aad51593ade1e47ab6cced4653e
# PCR2=21b9efbc184807662e966d34f390821309eeac6802309798826296bf3e8bec7c10edb30948c90ba67310f7b964fc500a
```

Every build from same source produces identical PCRs.

## On-Chain Integration

### Register Enclave

1. **Deploy enclave package**:
```bash
sui client switch --env testnet
cd move/enclave
sui move build
sui client publish
# Record ENCLAVE_PACKAGE_ID
```

2. **Deploy dApp logic**:
```bash
cd move/<APP>
sui move build
sui client publish
# Record CAP_OBJECT_ID, ENCLAVE_CONFIG_OBJECT_ID, APP_PACKAGE_ID
```

3. **Update PCRs on-chain**:
```bash
sui client call --function update_pcrs \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --type-args "$APP_PACKAGE_ID::$MODULE_NAME::$OTW_NAME" \
  --args $ENCLAVE_CONFIG_OBJECT_ID $CAP_OBJECT_ID 0x$PCR0 0x$PCR1 0x$PCR2
```

4. **Register enclave with attestation**:
```bash
sh register_enclave.sh $ENCLAVE_PACKAGE_ID $APP_PACKAGE_ID $ENCLAVE_CONFIG_OBJECT_ID $ENCLAVE_URL $MODULE_NAME $OTW_NAME
# Record ENCLAVE_OBJECT_ID
```

### Using Verified Computation

1. **Request computation from enclave**:
```bash
curl -H 'Content-Type: application/json' \
  -d '{"payload": {"location": "San Francisco"}}' \
  -X POST http://<PUBLIC_IP>:3000/process_data
```

2. **Submit result to Move contract**:
```bash
sh update_weather.sh \
  $APP_PACKAGE_ID \
  $MODULE_NAME \
  $OTW_NAME \
  $ENCLAVE_OBJECT_ID \
  "<signature>" \
  <timestamp> \
  "<location>" \
  <temperature>
```

### Enclave Management

**Multiple Instances**:
- One `EnclaveConfig` defines PCRs (version control)
- Multiple `Enclave` objects represent instances with unique public keys
- All instances use same `config_version` for consistency
- Admin can register/destroy `Enclave` objects

**Updating PCRs**:
- EnclaveCap holder can update PCRs when server code changes
- Get new PCRs: `make ENCLAVE_APP=<APP> && cat out/nitro.pcrs`
- Reuse registration steps to update on-chain

### Signing Payloads

Payloads use Binary Canonical Serialization (BCS):
- Must match structure in enclave Rust code
- Write unit tests in both Move and Rust
- Ensure consistency between signing and verification
- Mismatch causes verification failure in `enclave.move`

## Use Cases

### Trusted Oracles
- Process off-chain data from Web2 services (weather, sports, financial)
- Access decentralized storage (Walrus) in tamper-resistant way
- Provide verified data feeds to smart contracts

### AI Agents
- Securely run AI models for inference
- Execute agentic workflows with provenance
- Provide model output with on-chain attestation
- Data and model provenance verification

### DePIN Solutions
- Private data computation for IoT networks
- Supply chain data processing
- Decentralized Physical Infrastructure management

### Fraud Prevention
- DEX order matching and settlement
- Layer-2 collision and fraud prevention
- Secure computation between untrusted parties
- Multi-party computation scenarios

### Identity Management
- On-chain verifiability for decentralized governance
- Proof of tamper resistance
- Privacy-preserving identity verification

### Combined with Seal
Powerful privacy-preserving use cases:
- Nautilus: Secure and verifiable computation
- Seal: Secure key access control
- Challenge: Persist secret keys across TEE restarts
- Solution: Seal stores keys, grants access only to attested TEEs
- Application: Shared encrypted state with private request processing

## TEE Security Considerations

### Cloud-Based TEE Benefits

**AWS Nitro Enclaves chosen for**:
1. **Quick vulnerability response**:
   - Providers receive early security signals
   - Can patch efficiently

2. **Strong physical security**:
   - Tightly controlled data center access
   - Reduces hardware attack risk

3. **Compliance standards**:
   - Regular audits: SOC 2, ISO 27001, CSA STAR
   - Ensures operational integrity

**Evaluation**: Consider if trust model aligns with your application's threat profile and security needs.

### Trust Assumptions

Users must trust:
- AWS as root certificate authority
- TEE provider's attestation mechanism
- Developer's published source code (if using reproducible builds)
- Package deployer for access control policies

### Limitations

Template is:
- Starting point for building own enclave
- Not feature complete
- Has NOT undergone security audit
- Offered "AS IS" for evaluation purposes
- Apache 2.0 licensed for modification

## Troubleshooting

### Common Issues

**Traffic forwarder error**:
- Ensure all domains in `allowed_endpoints.yaml`
- Test with health check endpoint

**Docker not running**:
- EC2 instance may still be starting
- Wait and retry

**Cannot connect to enclave**:
- VSOCK communication issue
- Verify enclave running: `sh expose_enclave.sh`

**Reset enclave**:
```bash
cd nautilus/
sh reset_enclave.sh
# Then rebuild image
```

### Debug Mode

Run enclave in debug mode (prints all logs):
```bash
make run-debug  # Instead of make run
```

**Note**: Debug mode PCR values are all zeros and NOT valid for production.

## Advanced Configuration

### Application Load Balancer (Optional)
- Set up ALB for EC2 instance
- Configure SSL/TLS certificate from AWS Certificate Manager
- Set up Amazon Route 53 for DNS routing

### External Domain Access
- Add domains to `allowed_endpoints.yaml`
- Rerun `configure_enclave.sh` to regenerate instance
- Endpoint list compiled into enclave build

### Secrets Management
- Create secret in AWS Secrets Manager
- Pass as environment variable to enclave
- Avoid including in codebase
- Access from Secrets Manager console

## FAQs

### Why AWS Nitro Enclaves initially?
- Maturity and reproducible build support
- Additional TEE providers may be added based on community feedback
- Contact Nautilus team on Sui Discord for suggestions

### Where is AWS root of trust?
Stored in Sui framework for attestation verification:

```bash
# Download from Sui repo
curl https://raw.githubusercontent.com/MystenLabs/sui/refs/heads/main/crates/sui-types/src/nitro_root_certificate.pem -o cert_sui.pem
sha256sum cert_sui.pem
# 6eb9688305e4bbca67f44b59c29a0661ae930f09b5945b5d1d9ae01125c8d6c0

# Download from AWS
curl https://aws-nitro-enclaves.amazonaws.com/AWS_NitroEnclaves_Root-G1.zip -o cert_aws.zip
unzip cert_aws.zip
sha256sum root.pem
# 6eb9688305e4bbca67f44b59c29a0661ae930f09b5945b5d1d9ae01125c8d6c0
```

Hashes must match for verification.

## Best Practices

### Development
- Test most functionality locally before deploying to enclave
- Use unit tests for payload signing consistency (Move and Rust)
- Version your shared objects for secure upgrades
- Keep trusted computing base minimal

### Security
- Verify attestation documents on-chain during registration only
- Use reproducible builds for transparency
- Publish source code publicly when possible
- Regularly update to latest security patches

### Operations
- Set up proper monitoring and alerting
- Implement backup and disaster recovery
- Use load balancers for high availability
- Plan for key rotation and updates

### Performance
- Cache responses when appropriate
- Minimize round trips to TEE
- Use batch processing where possible
- Optimize external API calls

## Future Plans

Nautilus will expand to support:
- Additional TEE providers (community input welcome)
- More integration patterns
- Enhanced developer tools
- Expanded example applications

**Not Goals**:
- Native TEE network (partners may provide)
- Users deploy and manage own TEEs
- Some partners may offer managed TEE networks

## Resources

- GitHub repository with templates
- Sui Discord: Nautilus team channel
- Integration examples (weather, Twitter)
- Documentation and guides
- Community discussions and support
