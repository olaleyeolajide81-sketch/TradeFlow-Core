## Contract Versioning

TradeFlow-Core follows [Semantic Versioning](https://semver.org/) (SemVer) for all on-chain contract releases.

### Format

```
vMAJOR.MINOR.PATCH
```

| Segment | When to increment |
|---------|-------------------|
| MAJOR   | Breaking changes to public interfaces or storage layout |
| MINOR   | Backward-compatible new features or functions |
| PATCH   | Bug fixes and security patches with no interface changes |

**Current version:** `v1.0.0`

### How It Works

The version string is defined as a compile-time constant and written to instance storage once during `initialize_factory`. It cannot be changed after deployment.

```rust
const CONTRACT_VERSION: &str = "v1.0.0";
```

### Reading the Version

Frontends, indexers, and client integrations should call `get_version()` before interacting with the contract to confirm they are talking to the expected version.

```rust
// Soroban SDK call
let version: String = client.get_version();
// Returns: "v1.0.0"
```

### Upgrade Policy

When a new contract version is deployed:
- The `CONTRACT_VERSION` constant is updated in the source code before deployment.
- `initialize_factory` writes the new version string to storage on first call.
- The previous contract version remains readable from the old instance until migration.
- Clients should always verify the version before executing transactions.