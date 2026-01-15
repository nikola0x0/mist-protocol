# [Sprint] Hung - Infrastructure & AWS Deployment

**Owner:** @hung
**Epic:** TEE Attestation & AWS Deployment

---

## 🎯 Goals

1. Provision AWS EC2 with Nitro Enclave support
2. Configure Docker image for enclave deployment
3. Generate Nautilus attestation and register TEE onchain
4. Automate deployment with systemd services
5. Document deployment procedures

---

## 📋 Tasks

### Story 3.1: AWS EC2 & Nitro Setup

**Goal:** Provision EC2 instance with Nitro Enclave environment

- [ ] Provision EC2 instance (c5.xlarge or c5.2xlarge)
- [ ] Enable Nitro Enclave support in instance settings
- [ ] Configure security groups:
  - Inbound: 443 (HTTPS) from 0.0.0.0/0
  - Inbound: 22 (SSH) from your IP only
  - Outbound: 443 (HTTPS) for Sui RPC and SEAL servers
- [ ] SSH into instance and install Nitro CLI
- [ ] Configure enclave allocator (4GB RAM, 2 vCPUs)
- [ ] Test enclave allocator status

**Verification:**
```bash
ssh -i ~/.ssh/mist-protocol.pem ec2-user@<instance-ip>
nitro-cli --version
sudo systemctl status nitro-enclaves-allocator
```

**Success:**
- SSH works
- `nitro-cli --version` succeeds
- Allocator status: active

---

### Story 3.2: Enclave Docker Configuration

**Goal:** Create Docker image for enclave deployment

- [ ] Create `backend/Dockerfile.enclave` (minimal Debian base)
- [ ] Write `backend/enclave-entrypoint.sh` for vsock communication
- [ ] Build Docker image locally
- [ ] Build EIF file with `nitro-cli build-enclave`
- [ ] Test enclave launch with `nitro-cli run-enclave`
- [ ] Verify enclave running with `nitro-cli describe-enclaves`

**Files to Create:**
- `backend/Dockerfile.enclave`
- `backend/enclave-entrypoint.sh`

**Success:**
- EIF file builds successfully
- Enclave runs without errors
- `describe-enclaves` shows running enclave

**Coordination:** With Max - need backend binary that compiles

---

### Story 3.3 (Shared with Max): Generate Nautilus Attestation & Register TEE

**Goal:** Generate enclave keypair, create attestation, register onchain

**Hung's Tasks (Infrastructure):**
- [ ] Generate enclave Ed25519 keypair during NSM initialization
- [ ] Seal keypair to persistent storage (survives restarts)
- [ ] Generate NSM attestation document including public key
- [ ] Extract PCR measurements from attestation
- [ ] Document PCR values for verification

**Max's Tasks (Backend Code):**
- See Max's issue for backend implementation

**Files to Modify:**
- `backend/src/common.rs`
- `backend/src/apps/mist-protocol/swap_executor.rs`
- `backend/Cargo.toml`
- `backend/enclave-entrypoint.sh`

**Success:**
- Enclave generates and seals keypair
- Attestation document includes public key + PCRs
- TEE enclave registered onchain
- PCR values documented

**Important:** Keypair NEVER leaves enclave. Signing happens inside TEE.

**Resources:**
- [Nautilus Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Using Nautilus](https://docs.sui.io/concepts/cryptography/nautilus/using-nautilus)

**Coordination:** With Nikola on `register_tee_enclave()` Move function

---

### Story 3.4: Deployment Automation

**Goal:** Automate enclave deployment with systemd services

- [ ] Create systemd service: `systemd/mist-enclave.service`
  - Auto-start enclave on boot
  - Restart on failure
- [ ] Create systemd service: `systemd/mist-vsock-proxy.service`
  - Proxy vsock to TCP (port 3001)
  - Auto-start after enclave
- [ ] Write deployment script: `scripts/deploy.sh`
  - Build EIF locally
  - Upload to EC2
  - Stop old enclave
  - Start new enclave
  - Verify attestation
- [ ] Test deployment from local machine
- [ ] Test systemd services (enable, start, status)

**Files to Create:**
- `scripts/deploy.sh`
- `systemd/mist-enclave.service`
- `systemd/mist-vsock-proxy.service`

**Success:**
- `./scripts/deploy.sh` deploys new enclave
- Services auto-start on boot
- Enclave restarts automatically on crash

---

### Story 3.5: Deployment Documentation

**Goal:** Document deployment procedures and troubleshooting

- [ ] Write `docs/DEPLOYMENT.md`
  - Prerequisites
  - Step-by-step deployment guide
  - Verification steps
- [ ] Write `docs/TROUBLESHOOTING.md`
  - Common issues and solutions
  - Debugging commands
  - Log locations
- [ ] Create security hardening checklist
- [ ] Document PCR values for each build

**Files to Create:**
- `docs/DEPLOYMENT.md`
- `docs/TROUBLESHOOTING.md`

**Success:** Another team member can follow docs and deploy successfully

---

## 🔗 Coordination Points

### With Max:
- **Backend Binary** - After Story 3.2
  - Max provides unified backend binary
  - Test Docker build with Max's binary
  - Verify enclave runs

- **Keypair Generation** - Story 3.3
  - Coordinate on enclave initialization
  - Test signing works inside enclave
  - Verify attestation generation

### With Nikola:
- **TEE Registration** - After Story 3.3
  - Provide attestation document format
  - Test registration transaction
  - Verify TEE public key stored onchain

---

## ✅ Definition of Done

- [ ] EC2 instance running with Nitro Enclave support
- [ ] Enclave Docker image builds successfully
- [ ] Enclave EIF deploys and runs
- [ ] TEE keypair generated inside enclave (sealed)
- [ ] Attestation document generated with PCRs
- [ ] TEE registered onchain via Move contract
- [ ] `deploy.sh` script works end-to-end
- [ ] Systemd services configured and tested
- [ ] Deployment documentation complete
- [ ] PCR values documented

---

## 📚 Files to Create

- `backend/Dockerfile.enclave`
- `backend/enclave-entrypoint.sh`
- `scripts/deploy.sh`
- `systemd/mist-enclave.service`
- `systemd/mist-vsock-proxy.service`
- `docs/DEPLOYMENT.md`
- `docs/TROUBLESHOOTING.md`

## 📚 Files to Modify

- `backend/src/common.rs` (with Max)
- `backend/enclave-entrypoint.sh`

---

## 📖 Resources

- [AWS Nitro Enclaves Documentation](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave.html)
- [Nautilus Documentation](https://docs.sui.io/concepts/cryptography/nautilus)
- [Nautilus Design](https://docs.sui.io/concepts/cryptography/nautilus/nautilus-design)

---

## 🚀 Quick Reference Commands

```bash
# Check Nitro CLI
nitro-cli --version
sudo systemctl status nitro-enclaves-allocator

# Build enclave
docker build -f backend/Dockerfile.enclave -t mist-backend .
nitro-cli build-enclave --docker-uri mist-backend --output-file mist.eif

# Run enclave
nitro-cli run-enclave --eif-path mist.eif --memory 4096 --cpu-count 2 --enclave-cid 16

# Check enclave status
nitro-cli describe-enclaves

# Deploy
./scripts/deploy.sh
```

---

**Estimated Complexity:** High (new to project, but well-documented)
**Blocked By:** Max's unified backend binary (for Story 3.2)
**Can Start:** Story 3.1 immediately
