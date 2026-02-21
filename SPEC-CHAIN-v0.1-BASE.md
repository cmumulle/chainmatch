# ACE CHAIN — On-Chain Receipt Contract Specification v0.1

## Base (EVM) — Solidity + EIP-712 + Foundry

---

## 1. DESIGN PRINCIPLES

### 1.1 Trust Model (Explicit)

**Results are tamper-evident and publicly auditable, but depend on the authorized signer set.**

The game server is authoritative for match simulation. The blockchain serves as:
- An **immutable receipt ledger** — once recorded, results cannot be altered or deleted.
- A **public audit trail** — anyone can read all match results without needing the game's API.
- A **signer attestation** — the authorized match server cryptographically attests to each result.

The blockchain does **NOT** provide:
- Trustless verification of gameplay (the server could theoretically fabricate results).
- Independent re-simulation (floating point determinism is not guaranteed across platforms).

Mitigation for trust concerns is handled through **signer governance** (Section 6).

### 1.2 Slim Envelope Design

**On-chain storage is minimal.** The contract stores only:
- `matchId → envelopeHash` (duplicate prevention + integrity anchor)

**Everything else goes into emitted events**, which are:
- Cheap to emit (log storage is far cheaper than contract storage on EVM)
- Indexed by any off-chain service
- Permanently available via archive nodes

**Full match data** (replays, detailed stats) is stored off-chain with only a hash on-chain.

### 1.3 Generic Receipt Schema

The contract is designed as a **generic achievement receipt system** — not tennis-specific. This allows future sports (basketball, football, racing) to use the same contract.

Tennis-specific data is encoded in `resultData` (opaque bytes) and `resultHash` (hash of structured result JSON/protobuf). The contract does not parse sport-specific fields.

---

## 2. NETWORK DETAILS

| Parameter | Testnet | Mainnet |
|-----------|---------|---------|
| Network | Base Sepolia | Base Mainnet |
| Chain ID | 84532 | 8453 |
| RPC URL | `https://sepolia.base.org` | `https://mainnet.base.org` |
| Explorer | `https://sepolia.basescan.org` | `https://basescan.org` |
| Finality | OP Stack: unsafe → safe → finalized | Same |
| Avg tx fee | Free (testnet) | Low-cent range (variable) |

**Finality UX model:**
- Transactions appear in "unsafe" blocks within seconds.
- "Safe" confirmation follows quickly on Base.
- Full "finalized" state is tied to Ethereum L1 finality (~20–30 min, not 7 days).
- For prototype: display tx hash immediately after submission. Note finality status is informational.

---

## 3. SOLIDITY INTERFACE

### 3.1 Contract: AceChainReceipts.sol

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

/// @title AceChainReceipts
/// @notice Immutable match result receipts for ACE CHAIN sports games.
/// @dev Generic receipt system — not sport-specific. Sport-specific data
///      is encoded in resultData/resultHash fields.
contract AceChainReceipts is EIP712, Ownable2Step, Pausable {
    using ECDSA for bytes32;

    // ─── Types ───────────────────────────────────────────────────────

    /// @notice A match result envelope signed by an authorized game server.
    struct MatchEnvelope {
        bytes16 matchId;        // Unique match identifier (UUID bytes)
        bytes32 gameId;         // Game identifier (e.g., keccak256("ACE_TENNIS"))
        bytes32 playerA;        // Player A identity hash
        bytes32 playerB;        // Player B identity hash
        bytes32 winner;         // Winner identity hash (playerA, playerB, or 0x0 for draw/retirement)
        bytes32 resultHash;     // SHA-256 of canonical result JSON (scores, stats)
        bytes32 replayHash;     // SHA-256 of replay binary file
        uint8   matchType;      // 0 = friendly, 1 = ranked, 2 = tournament
        uint32  durationSecs;   // Match duration in seconds
        uint48  endedAt;        // Unix timestamp when match ended
        uint16  gameVersion;    // Client/server version at match time
    }

    // ─── Storage ─────────────────────────────────────────────────────

    /// @notice matchId → keccak256(abi.encode(envelope)) for duplicate prevention
    mapping(bytes16 => bytes32) public receipts;

    /// @notice Authorized signer addresses (game servers)
    mapping(address => bool) public authorizedSigners;

    /// @notice Total number of recorded matches
    uint256 public matchCount;

    // ─── Events ──────────────────────────────────────────────────────

    /// @notice Emitted for every recorded match. This is the primary data record.
    ///         Off-chain indexers should consume this event to build leaderboards,
    ///         match histories, and rating calculations.
    event MatchRecorded(
        bytes16 indexed matchId,
        bytes32 indexed gameId,
        bytes32 indexed playerA,
        bytes32         playerB,
        bytes32         winner,
        bytes32         resultHash,
        bytes32         replayHash,
        uint8           matchType,
        uint32          durationSecs,
        uint48          endedAt,
        uint16          gameVersion,
        address         signer
    );

    /// @notice Emitted when a signer is added or removed
    event SignerUpdated(address indexed signer, bool authorized);

    // ─── Errors ──────────────────────────────────────────────────────

    error DuplicateMatch(bytes16 matchId);
    error UnauthorizedSigner(address recovered);
    error InvalidSignature();
    error EmptyBatch();
    error BatchTooLarge();
    error ZeroMatchId();

    // ─── Constants ───────────────────────────────────────────────────

    /// @dev Maximum batch size to prevent gas limit issues
    uint256 public constant MAX_BATCH_SIZE = 200;

    /// @dev EIP-712 typehash for MatchEnvelope
    bytes32 public constant MATCH_ENVELOPE_TYPEHASH = keccak256(
        "MatchEnvelope("
            "bytes16 matchId,"
            "bytes32 gameId,"
            "bytes32 playerA,"
            "bytes32 playerB,"
            "bytes32 winner,"
            "bytes32 resultHash,"
            "bytes32 replayHash,"
            "uint8 matchType,"
            "uint32 durationSecs,"
            "uint48 endedAt,"
            "uint16 gameVersion"
        ")"
    );

    // ─── Constructor ─────────────────────────────────────────────────

    constructor(
        address initialOwner,
        address initialSigner
    )
        EIP712("AceChainReceipts", "1")
        Ownable(initialOwner)
    {
        authorizedSigners[initialSigner] = true;
        emit SignerUpdated(initialSigner, true);
    }

    // ─── External Functions ──────────────────────────────────────────

    /// @notice Record a single match result.
    /// @param envelope The match result data.
    /// @param signature EIP-712 ECDSA signature from an authorized signer.
    function recordMatch(
        MatchEnvelope calldata envelope,
        bytes calldata signature
    ) external whenNotPaused {
        _recordSingle(envelope, signature);
    }

    /// @notice Record multiple match results in one transaction (batching).
    /// @param envelopes Array of match result data.
    /// @param signatures Array of EIP-712 ECDSA signatures (same order as envelopes).
    function recordMatches(
        MatchEnvelope[] calldata envelopes,
        bytes[] calldata signatures
    ) external whenNotPaused {
        uint256 len = envelopes.length;
        if (len == 0) revert EmptyBatch();
        if (len > MAX_BATCH_SIZE) revert BatchTooLarge();
        if (len != signatures.length) revert("length mismatch");

        for (uint256 i = 0; i < len; ) {
            _recordSingle(envelopes[i], signatures[i]);
            unchecked { ++i; }
        }
    }

    // ─── Admin Functions ─────────────────────────────────────────────

    /// @notice Add or remove an authorized signer.
    /// @dev Only callable by contract owner (multi-sig in production).
    function setSigner(address signer, bool authorized) external onlyOwner {
        authorizedSigners[signer] = authorized;
        emit SignerUpdated(signer, authorized);
    }

    /// @notice Pause receipt recording (emergency use).
    function pause() external onlyOwner {
        _pause();
    }

    /// @notice Resume receipt recording.
    function unpause() external onlyOwner {
        _unpause();
    }

    // ─── View Functions ──────────────────────────────────────────────

    /// @notice Check if a match has been recorded.
    function isMatchRecorded(bytes16 matchId) external view returns (bool) {
        return receipts[matchId] != bytes32(0);
    }

    /// @notice Get the envelope hash for a recorded match.
    function getEnvelopeHash(bytes16 matchId) external view returns (bytes32) {
        return receipts[matchId];
    }

    /// @notice Verify an envelope matches its on-chain hash.
    /// @return True if the envelope data matches what was recorded.
    function verifyEnvelope(
        MatchEnvelope calldata envelope
    ) external view returns (bool) {
        bytes32 stored = receipts[envelope.matchId];
        if (stored == bytes32(0)) return false;
        return stored == _hashEnvelope(envelope);
    }

    /// @notice Get the EIP-712 domain separator (for off-chain signing).
    function domainSeparator() external view returns (bytes32) {
        return _domainSeparatorV4();
    }

    // ─── Internal Functions ──────────────────────────────────────────

    function _recordSingle(
        MatchEnvelope calldata envelope,
        bytes calldata signature
    ) internal {
        // Validate
        if (envelope.matchId == bytes16(0)) revert ZeroMatchId();
        if (receipts[envelope.matchId] != bytes32(0)) {
            revert DuplicateMatch(envelope.matchId);
        }

        // Recover signer from EIP-712 typed data signature
        bytes32 structHash = _structHash(envelope);
        bytes32 digest = _hashTypedDataV4(structHash);
        address recovered = ECDSA.recover(digest, signature);

        if (!authorizedSigners[recovered]) {
            revert UnauthorizedSigner(recovered);
        }

        // Store envelope hash (minimal storage)
        bytes32 envelopeHash = _hashEnvelope(envelope);
        receipts[envelope.matchId] = envelopeHash;
        unchecked { ++matchCount; }

        // Emit full envelope as event (cheap, indexable)
        emit MatchRecorded(
            envelope.matchId,
            envelope.gameId,
            envelope.playerA,
            envelope.playerB,
            envelope.winner,
            envelope.resultHash,
            envelope.replayHash,
            envelope.matchType,
            envelope.durationSecs,
            envelope.endedAt,
            envelope.gameVersion,
            recovered
        );
    }

    function _structHash(
        MatchEnvelope calldata envelope
    ) internal pure returns (bytes32) {
        return keccak256(abi.encode(
            MATCH_ENVELOPE_TYPEHASH,
            envelope.matchId,
            envelope.gameId,
            envelope.playerA,
            envelope.playerB,
            envelope.winner,
            envelope.resultHash,
            envelope.replayHash,
            envelope.matchType,
            envelope.durationSecs,
            envelope.endedAt,
            envelope.gameVersion
        ));
    }

    function _hashEnvelope(
        MatchEnvelope calldata envelope
    ) internal pure returns (bytes32) {
        return keccak256(abi.encode(envelope));
    }
}
```

### 3.2 Dependencies (Foundry)

```toml
# foundry.toml
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc = "0.8.24"
optimizer = true
optimizer_runs = 10000

[rpc_endpoints]
base_sepolia = "${BASE_SEPOLIA_RPC_URL}"
base_mainnet = "${BASE_MAINNET_RPC_URL}"

[etherscan]
base_sepolia = { key = "${BASESCAN_API_KEY}", url = "https://api-sepolia.basescan.org/api" }
base_mainnet = { key = "${BASESCAN_API_KEY}", url = "https://api.basescan.org/api" }
```

```bash
# Install OpenZeppelin contracts
forge install OpenZeppelin/openzeppelin-contracts --no-commit
```

Remappings (`remappings.txt`):
```
@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/
```

---

## 4. EIP-712 TYPED DATA LAYOUT

### 4.1 Domain

```json
{
    "name": "AceChainReceipts",
    "version": "1",
    "chainId": 84532,
    "verifyingContract": "0x<deployed_address>"
}
```

Chain ID changes per network (84532 for Base Sepolia, 8453 for Base Mainnet). The domain separator is computed on-chain via OpenZeppelin's `EIP712` base and must match the off-chain signer's domain.

### 4.2 Types

```json
{
    "MatchEnvelope": [
        { "name": "matchId",      "type": "bytes16" },
        { "name": "gameId",       "type": "bytes32" },
        { "name": "playerA",      "type": "bytes32" },
        { "name": "playerB",      "type": "bytes32" },
        { "name": "winner",       "type": "bytes32" },
        { "name": "resultHash",   "type": "bytes32" },
        { "name": "replayHash",   "type": "bytes32" },
        { "name": "matchType",    "type": "uint8" },
        { "name": "durationSecs", "type": "uint32" },
        { "name": "endedAt",      "type": "uint48" },
        { "name": "gameVersion",  "type": "uint16" }
    ]
}
```

### 4.3 Signing in Rust (alloy)

```rust
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::{eip712_domain, SolStruct};
use alloy::primitives::{Address, FixedBytes, U256};

// Define the EIP-712 struct in Rust matching the Solidity struct
alloy::sol! {
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct MatchEnvelope {
        bytes16 matchId;
        bytes32 gameId;
        bytes32 playerA;
        bytes32 playerB;
        bytes32 winner;
        bytes32 resultHash;
        bytes32 replayHash;
        uint8   matchType;
        uint32  durationSecs;
        uint48  endedAt;
        uint16  gameVersion;
    }
}

async fn sign_envelope(
    signer: &PrivateKeySigner,
    envelope: &MatchEnvelope,
    chain_id: u64,
    contract_address: Address,
) -> Vec<u8> {
    let domain = eip712_domain! {
        name: "AceChainReceipts",
        version: "1",
        chain_id: chain_id,
        verifying_contract: contract_address,
    };

    let signature = signer
        .sign_typed_data(envelope, &domain)
        .await
        .expect("signing failed");

    signature.as_bytes().to_vec()
}
```

### 4.4 Replay Protection

EIP-712 signatures include the chain ID and contract address in the domain separator. This prevents:
- **Cross-chain replay:** A signature for Base Sepolia cannot be used on Base Mainnet.
- **Cross-contract replay:** A signature for one contract deployment cannot be used on another.
- **Duplicate submission:** The contract stores `matchId → envelopeHash` and reverts if already recorded.

---

## 5. BATCHING STRATEGY

### 5.1 Prototype (v0.1)

One transaction per match via `recordMatch()`. Acceptable for testnet and low-volume launch.

### 5.2 Production (v0.2+)

The server accumulates match results and submits batches via `recordMatches()`.

**Batching parameters:**

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Target batch size | 25–100 | Balance latency vs. gas efficiency |
| Max batch size | 200 | Contract enforced; prevents gas limit issues |
| Max batch delay | 5 minutes | Don't make players wait too long for on-chain confirmation |
| Trigger | Whichever comes first: batch full OR delay elapsed | |

**Gas estimation (approximate):**

| Operation | Gas (approx) |
|-----------|-------------|
| Single `recordMatch()` | ~80,000–100,000 |
| Batch of 25 `recordMatches()` | ~1,500,000–2,000,000 |
| Batch of 100 | ~6,000,000–8,000,000 |
| Base Sepolia gas price | Variable, typically low |
| Base Mainnet per-match cost | Design for ~$0.001–$0.01 per match |

**Batching implementation (server-side Rust):**

```rust
pub struct BatchAccumulator {
    pending: Vec<(MatchEnvelope, Vec<u8>)>,  // (envelope, signature)
    max_size: usize,                          // 100
    max_delay: Duration,                      // 5 minutes
    last_flush: Instant,
}

impl BatchAccumulator {
    pub fn add(&mut self, envelope: MatchEnvelope, sig: Vec<u8>) -> Option<Batch> {
        self.pending.push((envelope, sig));
        if self.pending.len() >= self.max_size
            || self.last_flush.elapsed() >= self.max_delay
        {
            Some(self.flush())
        } else {
            None
        }
    }

    fn flush(&mut self) -> Batch {
        self.last_flush = Instant::now();
        Batch {
            envelopes: std::mem::take(&mut self.pending),
        }
    }
}
```

### 5.3 Indexing & Reorg Handling

The off-chain indexer reads `MatchRecorded` events from Base.

**OP Stack finality states to handle:**

| State | Meaning | Indexer behavior |
|-------|---------|-----------------|
| Unsafe | Block produced by sequencer | Show result in UI with "confirming" badge |
| Safe | Derived from L1 data | Show as "confirmed" |
| Finalized | Tied to finalized L1 block | Show as "finalized" ✓ |

**Reorg handling:**
- The indexer tracks block hashes.
- If a block hash changes (reorg), the indexer re-processes affected blocks.
- In practice, reorgs on Base are rare and typically shallow (1-2 blocks).
- For the prototype, ignore reorgs. For production, implement a simple reorg detector.

---

## 6. SIGNER GOVERNANCE & INCIDENT RESPONSE

### 6.1 Prototype (v0.1)

- Single EOA (externally owned account) as signer.
- Private key stored in `.env` file on developer machine.
- Acceptable risk: testnet only, no real value.

### 6.2 Production Architecture

```
┌─────────────────────────────────────────────┐
│          Contract Owner (Multi-Sig)          │
│    e.g., Gnosis Safe with 2-of-3 signers    │
│                                              │
│  Powers:                                     │
│  • Add/remove authorized signers             │
│  • Pause/unpause contract                    │
│  • No ability to modify recorded receipts    │
└─────────────────┬───────────────────────────┘
                  │ setSigner() / pause()
                  ▼
┌─────────────────────────────────────────────┐
│          AceChainReceipts Contract           │
│                                              │
│  authorizedSigners:                          │
│    [Server-EU-1]  ✓                         │
│    [Server-NA-1]  ✓                         │
│    [Server-Asia-1] ✓                        │
│    [Revoked-Old]  ✗                         │
└─────────────────────────────────────────────┘
```

**Signer key management:**
- Each game server region has its own signing key.
- Keys stored in cloud KMS (AWS KMS, GCP Cloud KMS, or Hashicorp Vault).
- Keys never exist in plaintext on disk.
- Key rotation: quarterly, or immediately if compromise suspected.

### 6.3 Incident Response: Signer Compromise

If a signer key is compromised:

1. **Immediate:** Owner multi-sig calls `setSigner(compromised, false)` to revoke.
2. **Immediate:** Owner calls `pause()` to halt all receipt recording.
3. **Investigation:** Identify time window of compromise. All receipts signed by the compromised key during the window are suspect.
4. **Remediation:** Generate new signer key in KMS. Owner calls `setSigner(newKey, true)` and `unpause()`.
5. **Communication:** Publish incident report. Flag suspect receipts in the off-chain indexer/API (add a `disputed` flag). Suspect receipts remain on-chain (immutable) but are annotated off-chain.

### 6.4 Contract Upgrade Model

**Prototype:** No upgrade mechanism. If the contract needs changes, deploy a new contract and update the client to point to it.

**Production:** Two options (decide before mainnet):
1. **Immutable + versioned:** Deploy new contract versions. Old receipts remain on old contract. Indexer reads from all versions.
2. **Proxy (UUPS/Transparent):** Upgradeable via owner multi-sig. More flexible but adds trust assumption.

Recommendation: **Option 1 (immutable + versioned)** for maximum trust. Receipts are permanent; contract logic is simple and unlikely to need changes.

---

## 7. OFF-CHAIN RESULT DATA

### 7.1 Canonical Result JSON (Hashed as `resultHash`)

The `resultHash` on-chain is SHA-256 of a canonical JSON structure. "Canonical" means: keys sorted, no extra whitespace, deterministic serialization.

```json
{
    "matchId": "550e8400-e29b-41d4-a716-446655440000",
    "gameId": "ACE_TENNIS",
    "players": [
        {
            "id": "a1b2c3...",
            "hero": 0,
            "heroName": "Viktor"
        },
        {
            "id": "d4e5f6...",
            "hero": 1,
            "heroName": "Mika"
        }
    ],
    "score": {
        "sets": [
            { "games": [6, 4] },
            { "games": [3, 6] },
            { "games": [7, 6], "tiebreak": [7, 5] }
        ],
        "winner": 0
    },
    "stats": [
        {
            "aces": 5,
            "doubleFaults": 2,
            "winners": 18,
            "unforcedErrors": 12,
            "firstServePct": 0.65,
            "pointsWon": 82,
            "longestRally": 24
        },
        {
            "aces": 2,
            "doubleFaults": 1,
            "winners": 14,
            "unforcedErrors": 15,
            "firstServePct": 0.72,
            "pointsWon": 76,
            "longestRally": 24
        }
    ],
    "surface": "hard",
    "matchType": "ranked",
    "durationSecs": 4230,
    "endedAt": 1708531200,
    "gameVersion": 1
}
```

**Canonical serialization rule:** Use `serde_json` with `to_string()` (compact, sorted keys). The hash must be reproducible by any JSON library that sorts keys and uses no trailing whitespace.

### 7.2 Replay Storage

| Aspect | Detail |
|--------|--------|
| Format | Bincode-serialized `ReplayV0` struct |
| Storage | Local filesystem (v0.1), Cloudflare R2 / S3 (v0.2+) |
| Path | `replays/{matchId}.replay` |
| Hash | SHA-256 of raw binary, stored as `replayHash` on-chain |
| Retention | Permanent for ranked matches, 30 days for friendly |
| Access | Public download via API: `GET /api/replays/{matchId}` |

### 7.3 Verification Procedure (Anyone Can Do This)

1. Find match on Basescan: search for `MatchRecorded` event with the `matchId`.
2. Read `resultHash` and `replayHash` from the event.
3. Download the result JSON from the game API: `GET /api/matches/{matchId}/result`.
4. Compute SHA-256 of the downloaded JSON. Compare to `resultHash`. Must match.
5. Download the replay file: `GET /api/replays/{matchId}`.
6. Compute SHA-256 of the replay binary. Compare to `replayHash`. Must match.
7. Verify the EIP-712 signature: recover the signer address from the event data + signature. Confirm the address is in the contract's `authorizedSigners` mapping.

---

## 8. FOUNDRY TEST SPECIFICATION

### 8.1 Test File: AceChainReceipts.t.sol

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test, console2} from "forge-std/Test.sol";
import {AceChainReceipts} from "../src/AceChainReceipts.sol";

contract AceChainReceiptsTest is Test {
    AceChainReceipts public receipts;

    address owner = makeAddr("owner");
    uint256 signerPk = 0xA11CE;
    address signer = vm.addr(signerPk);

    bytes16 constant MATCH_ID_1 = bytes16(uint128(1));
    bytes32 constant GAME_ID = keccak256("ACE_TENNIS");

    function setUp() public {
        vm.prank(owner);
        receipts = new AceChainReceipts(owner, signer);
    }

    // ── Core Recording ──────────────────────────────────

    function test_recordMatch_success() public { /* ... */ }
    function test_recordMatch_emitsEvent() public { /* ... */ }
    function test_recordMatch_storesEnvelopeHash() public { /* ... */ }
    function test_recordMatch_incrementsMatchCount() public { /* ... */ }

    // ── Duplicate Prevention ────────────────────────────

    function test_recordMatch_revertsDuplicate() public { /* ... */ }

    // ── Signature Verification ──────────────────────────

    function test_recordMatch_revertsUnauthorizedSigner() public { /* ... */ }
    function test_recordMatch_revertsInvalidSignature() public { /* ... */ }

    // ── Batching ────────────────────────────────────────

    function test_recordMatches_batchOf5() public { /* ... */ }
    function test_recordMatches_revertsEmptyBatch() public { /* ... */ }
    function test_recordMatches_revertsBatchTooLarge() public { /* ... */ }
    function test_recordMatches_partialDuplicateReverts() public { /* ... */ }

    // ── Admin ───────────────────────────────────────────

    function test_setSigner_addsNewSigner() public { /* ... */ }
    function test_setSigner_revokesSigner() public { /* ... */ }
    function test_setSigner_onlyOwner() public { /* ... */ }
    function test_pause_stopsRecording() public { /* ... */ }
    function test_unpause_resumesRecording() public { /* ... */ }
    function test_pause_onlyOwner() public { /* ... */ }

    // ── Verification ────────────────────────────────────

    function test_verifyEnvelope_validMatch() public { /* ... */ }
    function test_verifyEnvelope_unknownMatch() public { /* ... */ }
    function test_verifyEnvelope_tamperedData() public { /* ... */ }
    function test_isMatchRecorded() public { /* ... */ }

    // ── Edge Cases ──────────────────────────────────────

    function test_recordMatch_revertsZeroMatchId() public { /* ... */ }
    function test_recordMatch_whenPaused() public { /* ... */ }

    // ── Gas Benchmarks ──────────────────────────────────

    function test_gas_singleRecord() public { /* ... */ }
    function test_gas_batchOf25() public { /* ... */ }
    function test_gas_batchOf100() public { /* ... */ }

    // ── Helpers ──────────────────────────────────────────

    function _makeEnvelope(bytes16 matchId) internal pure returns (
        AceChainReceipts.MatchEnvelope memory
    ) {
        return AceChainReceipts.MatchEnvelope({
            matchId: matchId,
            gameId: GAME_ID,
            playerA: keccak256("playerA"),
            playerB: keccak256("playerB"),
            winner: keccak256("playerA"),
            resultHash: keccak256("result"),
            replayHash: keccak256("replay"),
            matchType: 1,
            durationSecs: 3600,
            endedAt: uint48(block.timestamp),
            gameVersion: 1
        });
    }

    function _signEnvelope(
        AceChainReceipts.MatchEnvelope memory envelope,
        uint256 pk
    ) internal view returns (bytes memory) {
        bytes32 structHash = keccak256(abi.encode(
            receipts.MATCH_ENVELOPE_TYPEHASH(),
            envelope.matchId,
            envelope.gameId,
            envelope.playerA,
            envelope.playerB,
            envelope.winner,
            envelope.resultHash,
            envelope.replayHash,
            envelope.matchType,
            envelope.durationSecs,
            envelope.endedAt,
            envelope.gameVersion
        ));

        bytes32 digest = keccak256(abi.encodePacked(
            "\x19\x01",
            receipts.domainSeparator(),
            structHash
        ));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, digest);
        return abi.encodePacked(r, s, v);
    }
}
```

### 8.2 Deployment Script: Deploy.s.sol

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {AceChainReceipts} from "../src/AceChainReceipts.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address owner = vm.envAddress("OWNER_ADDRESS");
        address signer = vm.envAddress("SIGNER_ADDRESS");

        vm.startBroadcast(deployerPk);

        AceChainReceipts receipts = new AceChainReceipts(owner, signer);
        console2.log("AceChainReceipts deployed at:", address(receipts));
        console2.log("Owner:", owner);
        console2.log("Initial signer:", signer);

        vm.stopBroadcast();
    }
}
```

**Deploy command:**
```bash
source .env
forge script script/Deploy.s.sol --rpc-url base_sepolia --broadcast --verify
```

---

## 9. FUTURE EXTENSIONS (Not in v0.1)

| Feature | Contract change needed |
|---------|----------------------|
| Multi-sport support | None — `gameId` field already generic |
| Token-gated tournaments | New contract or module, reads from receipt contract |
| Player wallet linking | Off-chain mapping; optionally add `playerAddress` field to envelope |
| On-chain leaderboard snapshot | Separate contract; periodically posts Merkle root of off-chain leaderboard |
| Dispute/challenge flow | New contract; references receipt by matchId, requires stake |
| NFT achievements/badges | Separate ERC-1155 contract; mints based on receipt events |

---

*End of Chain Specification v0.1 — Base (EVM)*
