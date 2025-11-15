# Nautilus Enclave Troubleshooting Guide

## Important: Should You Use Secrets?

### Recommendation: Always use secrets for realistic deployments

**Why secrets exist:**
- Store sensitive values (API keys, credentials, encryption keys) securely in AWS Secrets Manager
- Values are injected into the enclave at runtime via VSOCK
- Never appear in public code or container images
- Required for production use cases

**For weather-example:**
- Secret = WeatherAPI.com API key
- Can skip for quick testing, but adds complexity (see blocking issue below)

**For Mist Protocol:**
You'll likely need secrets for:
- Seal decryption credentials/keys
- DEX API authentication
- Private signing keys
- Any sensitive configuration

### The Problem with Skipping Secrets

When you answer **'n'** to the secret prompt in `configure_enclave.sh`:
- The `run.sh` script still has the VSOCK handshake code (line 31)
- It blocks waiting for secrets that never come
- The Nautilus server never starts
- Requires manual workarounds (sending empty JSON, modifying code)

### Recommended Approach

**Option 1: Use secrets (cleaner, production-ready)**
```bash
export KEY_PAIR=nautilus-demo-key
export AWS_PROFILE=nikola0x0-user
sh configure_enclave.sh weather-example

# Answer prompts:
Do you want to use a secret? (y/n): y
Do you want to create a new secret or use an existing secret ARN? (new/existing): new
Enter secret name: weather-api-key
Enter secret value: <your-api-key-from-weatherapi.com>
```

Get a free API key from: https://www.weatherapi.com/signup.aspx

**Complete Setup Flow (with secrets - RECOMMENDED - VERIFIED WORKING):**

This is the CORRECT flow that successfully runs the Nautilus weather-example enclave with proper secrets handling.

### Prerequisites
1. **IAM Permissions**: Your IAM user needs these policies:
   - `PowerUserAccess` (for general AWS resources)
   - Custom inline policy for IAM operations (see below)

**Required IAM Inline Policy** (attach to your IAM user):
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "NautilusIAMPermissions",
      "Effect": "Allow",
      "Action": [
        "iam:CreateRole",
        "iam:CreateInstanceProfile",
        "iam:AddRoleToInstanceProfile",
        "iam:AttachRolePolicy",
        "iam:PassRole",
        "iam:ListInstanceProfiles",
        "iam:GetRole",
        "iam:GetInstanceProfile",
        "iam:DeleteRole",
        "iam:DeleteInstanceProfile",
        "iam:RemoveRoleFromInstanceProfile",
        "iam:DetachRolePolicy"
      ],
      "Resource": "*"
    }
  ]
}
```

### Step-by-Step Instructions

```bash
# 1. Get your WeatherAPI key from https://www.weatherapi.com/signup.aspx
# Store it in your .env file: WEATHER_API_KEY=your-key-here

# 2. Configure and launch EC2 instance with secrets
export KEY_PAIR=nautilus-demo-key
export REGION=us-east-1
export AWS_PROFILE=nikola0x0-user
export EC2_INSTANCE_NAME=nautilus-weather

sh configure_enclave.sh weather-example
# Answer prompts:
# Do you want to use a secret? (y/n): y
# Do you want to create a new secret or use an existing secret ARN? (new/existing): new
# Enter secret name: weather-api-key
# Enter secret value: <paste-your-weatherapi-key>

# This will:
# - Create secret in AWS Secrets Manager
# - Create IAM role for EC2 to access secrets
# - Generate expose_enclave.sh and run.sh with proper secret handling
# - Launch EC2 instance
# - Associate IAM instance profile
# - Output: "ssh ec2-user@<PUBLIC_IP>"

# 3. Verify IAM instance profile was attached
aws ec2 describe-instances --profile $AWS_PROFILE --region $REGION \
  --instance-ids <INSTANCE_ID> \
  --query 'Reservations[0].Instances[0].[State.Name,PublicIpAddress,IamInstanceProfile.Arn]' \
  --output text
# Should show: running  <IP>  arn:aws:iam::...:instance-profile/role-nautilus-weather-...

# 4. If IAM profile is missing, attach Secrets Manager policy manually:
aws iam attach-role-policy --profile $AWS_PROFILE \
  --role-name role-nautilus-weather-<SUFFIX> \
  --policy-arn arn:aws:iam::aws:policy/SecretsManagerReadWrite

# 5. Wait 2-3 minutes for EC2 initialization
sleep 120

# 6. SSH into the instance
ssh -i ~/.ssh/nautilus-demo-key.pem ec2-user@<PUBLIC_IP>

# 7. Inside EC2: Clone the repo
git clone https://github.com/MystenLabs/nautilus.git
cd nautilus
exit  # Exit SSH to copy files

# 8. From LOCAL machine: Copy generated files
scp -i ~/.ssh/nautilus-demo-key.pem \
  src/nautilus-server/run.sh \
  ec2-user@<PUBLIC_IP>:~/nautilus/src/nautilus-server/run.sh

scp -i ~/.ssh/nautilus-demo-key.pem \
  expose_enclave.sh \
  ec2-user@<PUBLIC_IP>:~/nautilus/expose_enclave.sh

# IMPORTANT: Verify you copied the correct files!
# run.sh should have busybox commands and /nautilus-server at the end
# expose_enclave.sh should have aws secretsmanager and nitro-cli commands

# 9. SSH back and build enclave
ssh -i ~/.ssh/nautilus-demo-key.pem ec2-user@<PUBLIC_IP>
cd nautilus

# Build the enclave (includes the updated run.sh)
make ENCLAVE_APP=weather-example

# 10. Run enclave in screen (so it persists)
screen -S enclave
make run
# You should see: "Started enclave with enclave-cid: XX"
# Press Ctrl+A then D to detach from screen

# 11. Verify enclave is running
sudo nitro-cli describe-enclaves
# Should show State: "RUNNING" with a CID number

# 12. In ANOTHER SSH session: Expose the enclave
ssh -i ~/.ssh/nautilus-demo-key.pem ec2-user@<PUBLIC_IP>
cd nautilus
sudo sh expose_enclave.sh

# This will:
# - Fetch the secret from AWS Secrets Manager (requires IAM role!)
# - Send it to the enclave via VSOCK port 7777 (unblocks run.sh)
# - Start the socat proxy on port 3000

# 13. Verify socat is running correctly
ps aux | grep socat
# Should show: socat TCP4-LISTEN:3000,reuseaddr,fork VSOCK-CONNECT:<CID>:3000
# NOT: VSOCK-CONNECT:null:3000 (this means enclave wasn't running)

# 14. Test from your local machine
curl -H 'Content-Type: application/json' -X GET http://<PUBLIC_IP>:3000/health_check
# Expected: {"pk":"<enclave-public-key>","endpoints_status":{}}

curl -H 'Content-Type: application/json' \
  -d '{"payload": {"location": "San Francisco"}}' \
  -X POST http://<PUBLIC_IP>:3000/process_data
# Expected: {"response":{"intent":0,"timestamp_ms":...,"data":{"location":"San Francisco","temperature":13}},"signature":"..."}
```

### Success Indicators

✅ **Enclave is working if you see:**
- Health check returns JSON with `"pk"` field (enclave's public key)
- Process data returns weather data with `"temperature"` and cryptographic `"signature"`
- `ps aux | grep socat` shows correct CID (not `null`)

### Common Issues and Solutions

**Issue: "Unable to locate credentials" when running expose_enclave.sh**
- **Cause**: IAM instance profile not attached to EC2
- **Solution**: Run the manual `aws iam attach-role-policy` command from step 4

**Issue: socat shows `VSOCK-CONNECT:null:3000`**
- **Cause**: Enclave wasn't running when expose_enclave.sh executed
- **Solution**:
  1. `sudo pkill socat`
  2. Verify enclave: `sudo nitro-cli describe-enclaves`
  3. Rerun: `sudo sh expose_enclave.sh`

**Issue: Enclave exits immediately after `make run`**
- **Cause**: Wrong `run.sh` file was copied (contains `aws` or `nitro-cli` commands)
- **Solution**: Delete `src/nautilus-server/run.sh` on EC2 and re-copy from local machine
- **Then rebuild**: `make ENCLAVE_APP=weather-example`

**Issue: Empty reply from server on curl**
- **Cause**: Enclave's Nautilus server hasn't started yet (waiting for secrets)
- **Solution**: Run `expose_enclave.sh` in second SSH session to send secrets

### Files and Their Locations

| File | Location | Purpose |
|------|----------|---------|
| `run.sh` | EC2: `~/nautilus/src/nautilus-server/run.sh` | Runs INSIDE enclave, starts Nautilus server |
| `expose_enclave.sh` | EC2: `~/nautilus/expose_enclave.sh` | Runs on PARENT EC2, fetches secrets and starts proxy |
| `out/nitro.eif` | EC2: `~/nautilus/out/nitro.eif` | Enclave image file (built from run.sh + Rust code) |

**Critical**: `run.sh` gets baked into the EIF during `make`. If you change `run.sh`, you MUST rebuild!

**Option 2: Skip secrets (requires workarounds)**
- Only for quick testing
- Requires manual file copying and rebuilding
- See workaround below

## Issue: Enclave starts but doesn't respond to HTTP requests

### Symptoms
- `make run` completes successfully and returns to prompt
- `sudo nitro-cli describe-enclaves` shows enclave as "RUNNING"
- Enclave console shows "Script completed" but server never starts
- HTTP requests to port 3000 return "Connection reset by peer" or "Empty reply from server"
- socat proxy shows: `E connect(5, AF=40 cid:X port:3000, 16): Connection reset by peer`

### Root Cause
The `run.sh` script inside the enclave contains this blocking line:

```bash
JSON_RESPONSE=$(socat - VSOCK-LISTEN:7777,reuseaddr)
```

This waits indefinitely for secrets to be sent from the parent EC2 instance via VSOCK port 7777. If secrets are never sent, the script hangs here and never reaches the `/nautilus-server` line that actually starts the Rust application.

### Solution

**When running without secrets (answered 'n' to secret prompt):**

1. Ensure `configure_enclave.sh` properly modified `expose_enclave.sh` to send an empty JSON object:
   ```bash
   # In expose_enclave.sh, this line should exist:
   echo '{}' > secrets.json
   cat secrets.json | socat - VSOCK-CONNECT:$ENCLAVE_CID:7777
   ```

2. The modified files (`run.sh` and `expose_enclave.sh`) generated by `configure_enclave.sh` on your **local machine** must be:
   - Committed to git and pushed, then pulled on EC2, OR
   - Copied directly to EC2 using scp

3. **Copy modified files to EC2:**
   ```bash
   # From your local machine
   scp -i ~/.ssh/nautilus-demo-key.pem \
     src/nautilus-server/run.sh \
     ec2-user@<EC2_IP>:~/nautilus/src/nautilus-server/run.sh

   scp -i ~/.ssh/nautilus-demo-key.pem \
     expose_enclave.sh \
     ec2-user@<EC2_IP>:~/nautilus/expose_enclave.sh
   ```

4. **Rebuild the enclave** on EC2 (must rebuild after changing run.sh):
   ```bash
   sudo nitro-cli terminate-enclave --all
   make ENCLAVE_APP=weather-example
   make run-debug  # Use debug mode to see logs
   ```

5. **In a separate SSH session**, run expose_enclave.sh to send the empty secrets and start the proxy:
   ```bash
   sudo sh expose_enclave.sh
   ```

### Verification
After fixing, you should see in the debug console:
```
run.sh script is running
Script completed.
127.0.0.1   localhost
127.0.0.64   api.weatherapi.com
[Server logs showing Nautilus starting on port 3000]
```

Test with:
```bash
curl -H 'Content-Type: application/json' -X GET http://<EC2_IP>:3000/health_check
```

### Key Takeaways
- `configure_enclave.sh` generates code locally that MUST be present in the EC2 build
- The enclave image (EIF) is built from the code on EC2, not your local machine
- Always rebuild the enclave after modifying `run.sh` or any source code
- Use `make run-debug` to see actual logs when troubleshooting
- The secrets handshake via VSOCK port 7777 must complete before the server starts

## Common Mistakes
1. ❌ Running enclave on EC2 with code from official GitHub repo (not your modified version)
2. ❌ Forgetting to rebuild after copying new run.sh
3. ❌ Not running expose_enclave.sh (which sends the secrets to unblock run.sh)
4. ❌ Running expose_enclave.sh before the enclave is fully started
5. ❌ Using HTTPS instead of HTTP (enclaves serve HTTP on port 3000 by default)
