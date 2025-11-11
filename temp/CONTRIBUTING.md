# Contributing to Mist Protocol

## Team Workflow (2-Person Team)

Since we're a small team, we keep things lightweight but organized.

---

## GitHub Project Management

### Option 1: GitHub Issues (Recommended for 2 people)

**Why:** Simple, integrated, no overhead

**Setup:**
```bash
# We'll use labels for organization
Labels:
- `nautilus` - TEE/enclave work
- `frontend` - Next.js UI
- `backend` - Node.js API
- `contracts` - Move smart contracts
- `seal` - Seal integration
- `walrus` - Walrus storage
- `bug` - Something broken
- `blocked` - Waiting on something
- `high-priority` - Critical for demo
```

**Creating Issues:**
```markdown
Title: Implement stealth address generation

**Component:** frontend
**Priority:** high
**Assigned:** Max

## Description
Implement ECDH-based stealth address generation using @noble/curves

## Acceptance Criteria
- [ ] Generate ephemeral keypair
- [ ] Compute shared secret
- [ ] Derive stealth address
- [ ] Unit tests passing

## Related
- Depends on: #5 (wallet connection)
- Blocks: #7 (send payment UI)
```

**Workflow:**
1. Create issue for each task
2. Self-assign
3. Create branch: `feature/stealth-address`
4. Work on branch
5. PR when ready
6. Quick review (can self-merge for speed)
7. Close issue

### Option 2: GitHub Projects (Optional)

If you want a Kanban board:

**Columns:**
- Backlog
- In Progress
- In Review
- Done

**Setup:** Go to repo → Projects → New project → Board view

---

## Git Workflow

### Branch Strategy

```
main          # Always deployable
↓
develop       # Integration branch (optional for 2 people)
↓
feature/*     # Your work
```

**For 2 people, you can simplify:**
- Work on `feature/` branches
- PR directly to `main`
- Keep `main` clean

### Branch Naming

```bash
feature/stealth-address
feature/seal-integration
fix/wallet-connection-bug
chore/setup-aws
docs/architecture-diagram
```

### Commit Messages

```bash
# Format: <type>: <description>

feat: add stealth address generation
fix: resolve Seal session key expiry
docs: add Nautilus setup guide
chore: configure AWS Nitro instance
refactor: simplify encryption flow
test: add unit tests for stealth crypto

# With scope (optional)
feat(frontend): add wallet connection UI
fix(nautilus): handle attestation timeout
docs(architecture): update TEE diagram
```

### Example Workflow

```bash
# Start new feature
git checkout -b feature/nautilus-setup
git add nautilus/
git commit -m "feat(nautilus): initial AWS Nitro configuration"
git push origin feature/nautilus-setup

# Create PR on GitHub
# Review (or self-merge if urgent)
# Delete branch after merge
```

---

## Code Review Guidelines

### Quick Review Checklist
- [ ] Does it work? (test locally)
- [ ] Any security issues? (crypto, contracts)
- [ ] Is it documented?
- [ ] No secrets committed?

### When to Self-Merge
✅ Documentation changes
✅ Obvious bug fixes
✅ Your own component (not shared)

### When to Wait for Review
⏸ Smart contract changes
⏸ Cryptography implementation
⏸ Shared libraries
⏸ Breaking changes

**For hackathon speed: when in doubt, merge and iterate**

---

## Communication

### Daily Sync (5-10 minutes)
- What you did yesterday
- What you're doing today
- Any blockers

### Tools
- **Discord/Slack:** Quick questions
- **GitHub Issues:** Task tracking
- **Comments on PRs:** Technical discussion
- **Voice call:** When stuck (15 min limit)

### Asking for Help

Good:
> "Stuck on Seal session key expiry. Error: `SessionKeyExpired`. Tried X, Y. Any ideas?"

Bad:
> "Seal doesn't work"

---

## Development Guidelines

### Frontend (Max)
- Use TypeScript strict mode
- Components in `components/`
- Hooks in `hooks/`
- Client libraries in `lib/`
- Test wallet connection early

### Smart Contracts (Max)
- Keep contracts simple (MVP scope)
- Test on testnet frequently
- Document public functions
- Use shared objects for scanning

### Nautilus (You)
- Follow reference implementation pattern
- Document AWS setup steps
- Keep enclave code minimal
- Test attestation early

### Backend (Max)
- Keep simple for hackathon
- Can replace with Seal + Walrus later
- Focus on frontend first

---

## Testing Strategy

### Manual Testing (Primary for Hackathon)
- Test on testnet constantly
- Keep test wallets funded
- Document test flows
- Record issues in GitHub

### Automated Testing (Nice to have)
```bash
# Frontend
cd frontend && pnpm test

# Contracts
cd contracts && sui move test

# Nautilus
cd nautilus && cargo test
```

---

## Documentation

### What to Document
- Architecture decisions (in `docs/architecture/`)
- Setup instructions (in README or guides)
- API endpoints (if backend)
- Contract interfaces
- Known issues/blockers

### Format
- Markdown everywhere
- Code examples for complex parts
- Diagrams using Mermaid or ASCII

Example:
````markdown
## Stealth Address Generation

```typescript
function generateStealth(recipientScanKey: Uint8Array): StealthAddress {
  // Implementation
}
```

**Flow:**
1. Generate ephemeral keypair
2. Compute ECDH shared secret
3. Derive address from secret
4. Return stealth address + ephemeral public key
````

---

## Deployment

### Environments
- **Local:** Your machine
- **Testnet:** Sui testnet + AWS (staging)
- **Mainnet:** Post-hackathon

### Deploy Process
```bash
# Frontend (Vercel)
cd frontend && vercel deploy

# Contracts (Sui testnet)
cd contracts && sui client publish --gas-budget 100000000

# Nautilus (AWS)
cd nautilus && ./scripts/deploy.sh
```

---

## Project Milestones

### Week 1 (Nov 11-15)
- [ ] Repo structure ✅
- [ ] Frontend skeleton
- [ ] Contracts deployed
- [ ] Nautilus feasibility ✅
- [ ] AWS account setup

### Week 2 (Nov 18-22)
- [ ] Seal integration
- [ ] Stealth addresses
- [ ] Basic UI working
- [ ] Nautilus prototype

### Week 3 (Nov 25-29)
- [ ] Walrus integration
- [ ] Full user flow
- [ ] Demo preparation
- [ ] Documentation

---

## Hackathon-Specific Notes

### Speed vs Quality
- **Prioritize:** Working demo > perfect code
- **MVP mindset:** Cut features ruthlessly
- **Document shortcuts:** Add TODOs for post-hackathon

### When Things Break
1. Check GitHub issues for known problems
2. Ask in Sui Discord (#nautilus, #seal)
3. Fallback to simpler version
4. Document what you tried

### Scope Management
If behind schedule:
1. **Drop first:** Nautilus (use mock)
2. **Drop second:** Walrus (store on-chain)
3. **Keep always:** Stealth addresses + Seal
4. **Polish last:** UI can be basic

---

## Questions?

- **Git issues:** Ask Max or check Git docs
- **Nautilus questions:** Ask your teammate (Nautilus lead)
- **Seal/Walrus:** Both coordinate
- **Move contracts:** Max leads

---

## Emergency Contacts

- **Sui Discord:** #nautilus, #seal, #general
- **GitHub Issues:** Tag each other
- **Max:** [contact]
- **You:** [contact]

---

**Remember: Communication > Everything. Over-communicate blockers early!**
