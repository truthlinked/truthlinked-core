# truthlinked-core

Shared protocol types and utilities for TruthLinked. This crate is the single source of truth for on-chain data structures, hashing/signing domains, and manifest analysis helpers used across the node, CLI, and SDKs.

This crate is intentionally small and stable. It contains no network code, no storage backends, and no runtime execution. It is safe to depend on from clients and tooling.

**What’s inside**
- Canonical transaction types and intent enums
- Account and identity primitives
- Protocol constants that must stay consistent across all components
- WASM manifest analysis helpers used by SDKs and tooling

**What’s not inside**
- Consensus, P2P, or RPC logic
- Database or snapshot code
- Runtime execution or host functions
- Node-only constants (ports, batch intervals, etc.)

**Design goals**
- Single source of truth for protocol serialization
- Deterministic hashing and signing domains
- Minimal dependencies for downstream tooling
- No runtime side effects

**Usage**
Add the crate to your workspace or `Cargo.toml` and re-use the canonical types.

```toml
[dependencies]
truthlinked-core = "0.1.0"
```

Example: construct and sign a transaction intent using shared types.

```rust
use truthlinked_core::pq_execution::{
    AccountId, Transaction, TransactionIntent,
};
use truthlinked_core::pq_identity::AccountSecret;

fn build_example_tx(secret: &AccountSecret, chain_id: [u8; 32]) -> Transaction {
    let intent = TransactionIntent::Transfer {
        to: AccountId([0u8; 32]),
        amount: 1_000_000_000,
        memo: None,
    };

    let mut tx = Transaction::new(intent, chain_id);
    tx.sign(secret).expect("sign tx");
    tx
}
```

**Versioning**
This crate uses semantic versioning. Any breaking change to public types or constants will bump the major version.

**Security**
This crate does not handle keys directly. Any private key handling is delegated to higher-level crates that integrate secure storage.

**Contributing**
Keep changes minimal and backwards compatible. If a change affects the wire format or hashing, update all downstream crates that rely on this crate.

