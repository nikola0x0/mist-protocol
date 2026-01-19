# Mist Protocol - Troubleshooting Guide

Common issues and solutions when deploying the Mist Protocol TEE backend.

## Nitro Enclave Issues

### "Failed to start enclaves allocator"

**Symptoms:**
```
Job for nitro-enclaves-allocator.service failed
```

**Solutions:**

1. Check instance type supports Nitro Enclaves:
```bash
aws ec2 describe-instance-types \
  --instance-types c5.xlarge \
  --query 'InstanceTypes[0].NitroEnclavesSupport'
# Should return: "supported": "supported"
```

2. Check enclave was enabled at launch:
```bash
aws ec2 describe-instances \
  --instance-ids $INSTANCE_ID \
  --query 'Reservations[0].Instances[0].EnclaveOptions'
# Should show Enabled: true
```

3. If not enabled, terminate and re-launch with `--enclave-options 'Enabled=true'`

### "Not enough memory for enclave"

**Symptoms:**
```
Insufficient memory for enclave
```

**Solutions:**

1. Edit allocator config:
```bash
sudo vim /etc/nitro_enclaves/allocator.yaml
```

2. Reduce memory allocation:
```yaml
memory_mib: 2048  # Try smaller value
cpu_count: 2
```

3. Restart allocator:
```bash
sudo systemctl restart nitro-enclaves-allocator
```

### "Enclave CID already in use"

**Symptoms:**
```
CID 16 is already in use
```

**Solutions:**

1. List running enclaves:
```bash
nitro-cli describe-enclaves
```

2. Terminate existing enclave:
```bash
ENCLAVE_ID=$(nitro-cli describe-enclaves | jq -r '.[0].EnclaveID')
nitro-cli terminate-enclave --enclave-id $ENCLAVE_ID
```

3. Try running again:
```bash
make run
```

---

## Build Issues

### "Docker not running"

**Symptoms:**
```
Cannot connect to Docker daemon
```

**Solutions:**

1. Start Docker:
```bash
sudo systemctl start docker
```

2. Add user to docker group:
```bash
sudo usermod -aG docker ec2-user
```

3. Log out and back in, or run:
```bash
newgrp docker
```

### "Containerfile build fails"

**Symptoms:**
```
Error during build step
```

**Solutions:**

1. Clean build artifacts:
```bash
rm -rf out/
docker system prune -f
```

2. Check disk space:
```bash
df -h
# Need at least 10GB free
```

3. Rebuild:
```bash
make ENCLAVE_APP=mist-protocol
```

### "Feature not found: mist-protocol"

**Symptoms:**
```
error[E0277]: the trait bound `MistProtocol` is not satisfied
```

**Solutions:**

1. Verify Cargo.toml has feature:
```bash
grep "mist-protocol" src/nautilus-server/Cargo.toml
```

2. Verify lib.rs has module:
```bash
grep "mist_protocol" src/nautilus-server/src/lib.rs
```

---

## Network Issues

### "Can't reach enclave from outside"

**Symptoms:**
```
curl: (7) Failed to connect to <IP> port 3000
```

**Solutions:**

1. Check security group rules:
```bash
aws ec2 describe-security-groups --group-names mist-protocol-sg
# Verify port 3000 is open
```

2. Check vsock proxy is running:
```bash
ps aux | grep vsock
```

3. Re-run expose script:
```bash
sh expose_enclave.sh
```

4. Check firewall on instance:
```bash
sudo iptables -L -n
```

### "Traffic forwarder error"

**Symptoms:**
```
Error: Cannot reach api.example.com
```

**Solutions:**

1. Verify domain is in allowed_endpoints.yaml:
```bash
cat src/nautilus-server/src/apps/mist-protocol/allowed_endpoints.yaml
```

2. Add missing domain and re-run configure:
```bash
sh configure_enclave.sh mist-protocol
```

3. Rebuild enclave (domain list is compiled in):
```bash
make ENCLAVE_APP=mist-protocol
make run
```

### "SEAL key server unreachable"

**Symptoms:**
```
Error: Failed to connect to SEAL key server
```

**Solutions:**

1. Verify SEAL servers are in allowed_endpoints.yaml
2. Check SEAL server URLs are correct in seal_config.yaml
3. Verify SEAL servers are running (coordinate with team)

---

## Attestation Issues

### "Attestation verification failed"

**Symptoms:**
```
Error: PCR values don't match
```

**Solutions:**

1. Check you're running production mode (not debug):
```bash
# Debug mode has all-zero PCRs
# Use `make run` instead of `make run-debug`
```

2. Verify PCRs match onchain:
```bash
cat out/nitro.pcrs
# Compare with onchain EnclaveConfig object
```

3. If code changed, update PCRs onchain:
```bash
sui client call --function update_pcrs ...
```

### "NSM driver not available"

**Symptoms:**
```
Error: Failed to get attestation document
```

**Solutions:**

1. This only works inside Nitro Enclave
2. For local testing, use mock attestation
3. Verify enclave is actually running:
```bash
nitro-cli describe-enclaves
```

---

## Registration Issues

### "TEE registration failed"

**Symptoms:**
```
Error: Transaction failed during register_enclave
```

**Solutions:**

1. Verify all object IDs are correct:
```bash
echo $ENCLAVE_PACKAGE_ID
echo $ENCLAVE_CONFIG_OBJECT_ID
echo $CAP_OBJECT_ID
```

2. Verify you own the CAP object:
```bash
sui client objects --owner <your-address> | grep Cap
```

3. Check gas balance:
```bash
sui client gas
```

4. Verify enclave URL is reachable:
```bash
curl http://$ENCLAVE_URL:3000/get_attestation
```

---

## Log Locations

```bash
# Enclave console (debug mode)
# Run with: make run-debug

# Nitro CLI logs
journalctl -u nitro-enclaves-allocator

# Docker logs
docker logs <container-id>

# System logs
sudo dmesg | tail -100
```

## Debug Commands

```bash
# Check enclave status
nitro-cli describe-enclaves

# Check allocator
systemctl status nitro-enclaves-allocator

# Check memory allocation
cat /etc/nitro_enclaves/allocator.yaml

# Check network
netstat -tlnp | grep 3000

# Check vsock
ls -la /dev/vsock
```

## Getting Help

1. Check AWS Nitro Enclaves documentation
2. Review Nautilus docs: https://docs.sui.io/guides/developer/nautilus
3. Ask in team Slack channel
4. Open GitHub issue with:
   - Error message
   - Steps to reproduce
   - Instance type and region
   - Output of debug commands above
