Flow for the simplest case

Tweet: "@NautilusWallet send 5 SUI to @alice" 

Trigger to receive webhook information -> get id
Call Twitter API v2 to fetch tweet using the id from above
Get tweet information, get sender id, receiver id, create XWalletAccount if not exists for both parties, confirm sufficient 5 sui -> parse -> create transaction template -> create signature.

Need dapp to deposit funds, view transactions, deposit NFTs

Webhook
  |
  v
┌─────────────────────────────┐
│  Backend Service (Node/Rust)│
│  - Receive webhook          │
│  - Rate limiting            │
│  - Queue management         │
│  - Deduplication            │
│  - Retry logic              │
└──────────────┬──────────────┘
               |
               v
      Call /process_tweet
               |
               v
┌──────────────────────────────┐
│  Nautilus Enclave            │
│  - Fetch tweet               │
│  - Verify tweet              │
│  - Parse transfer            │
│  - Sign data                 │
│  - Return signature          │
└──────────────┬───────────────┘
               |
               v
        { data, signature }
               |
               v
┌──────────────────────────────┐
│  Backend Service             │
│  - Build transaction         │
│  - Submit to Sui             │
│  - Handle response           │
│  - Log/monitor               │
└──────────────────────────────┘