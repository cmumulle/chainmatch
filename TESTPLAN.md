# ACE CHAIN — Test Plan v0.1

---

## 1. TEST CATEGORIES

| Category | Scope | Runner | CI Required |
|----------|-------|--------|-------------|
| Unit (Rust) | Scoring, physics, types, hashing | `cargo test` | Yes |
| Unit (Solidity) | Contract logic, signatures, admin | `forge test` | Yes |
| Integration (Rust) | Client blockchain module ↔ Base Sepolia | `cargo test --test integration` | Optional (needs RPC) |
| End-to-End | Full match sim → replay → receipt → verify | Bash script | Manual |

---

## 2. UNIT TESTS — RUST

### 2.1 Scoring (`ace-shared/src/scoring.rs`)

Run: `cargo test -p ace-shared -- scoring`

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_initial_state` | Games [0,0], points Regular([0,0]), server 0, not complete |
| 2 | `test_point_progression` | 0→15→30→40 for player 0 |
| 3 | `test_game_won` | 40-0 + point → game to player, games [1,0], points reset |
| 4 | `test_deuce` | 40-40 → deuce state |
| 5 | `test_advantage` | Deuce + point → advantage |
| 6 | `test_advantage_lost` | Advantage + opponent point → back to deuce |
| 7 | `test_game_from_advantage` | Advantage + point → game won |
| 8 | `test_server_alternation` | After game won, server flips (0→1, 1→0) |
| 9 | `test_set_won_6_0` | 6 games to 0 → set complete, winner set |
| 10 | `test_set_won_6_4` | 6-4 → set complete |
| 11 | `test_no_set_at_5_4` | 5-4 is not set complete |
| 12 | `test_no_set_at_6_5` | 6-5 → set continues (need 7-5 or tiebreak) |
| 13 | `test_set_won_7_5` | 7-5 → set complete |
| 14 | `test_tiebreak_triggered` | 6-6 → GamePoints switches to Tiebreak |
| 15 | `test_tiebreak_won_7_5` | Tiebreak 7-5 → set complete, games show 7-6 |
| 16 | `test_tiebreak_extended` | Tiebreak 6-6 → continues → 8-6 wins |
| 17 | `test_tiebreak_server_alternation` | Server changes every 2 tiebreak points |
| 18 | `test_display_points_regular` | 30-15 → "30-15" |
| 19 | `test_display_points_deuce` | 40-40 → "Deuce" |
| 20 | `test_display_points_advantage` | AD-40 → "Ad-40" or "Advantage Player 1" |
| 21 | `test_full_set_simulation` | Random 1000 sets: all end correctly, invariants hold |

### 2.2 Physics Constants (`ace-shared/src/physics.rs`)

Run: `cargo test -p ace-shared -- physics`

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_ball_params_valid` | All params positive, gravity negative, max_speed > 0 |
| 2 | `test_court_dimensions` | Court length = 23.77, width = 8.23, etc. |
| 3 | `test_net_height` | Center 0.914, posts 1.067 |

### 2.3 Types (`ace-shared/src/types.rs`)

Run: `cargo test -p ace-shared -- types`

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_hero_stats_serialize_roundtrip` | Serialize → deserialize → identical |
| 2 | `test_shot_type_variants` | All 6 variants exist and serialize correctly |
| 3 | `test_court_surface_params` | Hard/Clay/Grass have distinct physics params |

### 2.4 Protocol (`ace-shared/src/protocol.rs`)

Run: `cargo test -p ace-shared -- protocol`

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_canonical_json_deterministic` | Same data → same JSON string, twice |
| 2 | `test_canonical_json_sorted_keys` | Keys appear in alphabetical order |
| 3 | `test_result_hash_deterministic` | Same data → same SHA-256 hash |
| 4 | `test_result_hash_changes_on_mutation` | Change one field → hash changes |
| 5 | `test_replay_hash` | SHA-256 of bincode bytes matches expected |

### 2.5 Blockchain Signer (`ace-client/src/blockchain/signer.rs`)

Run: `cargo test -p ace-client -- blockchain::signer`

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_eip712_domain_construction` | Domain separator matches expected value |
| 2 | `test_sign_envelope` | Signature is 65 bytes, recovers to correct address |
| 3 | `test_sign_envelope_deterministic` | Same input → same signature |
| 4 | `test_different_chain_id_different_sig` | Changing chain ID produces different signature |
| 5 | `test_different_contract_different_sig` | Changing contract address produces different signature |

---

## 3. UNIT TESTS — SOLIDITY (Foundry)

Run: `cd contracts && forge test -vv`

### 3.1 Core Recording

| # | Test | Assertion |
|---|------|-----------|
| 1 | `test_recordMatch_success` | No revert, returns normally |
| 2 | `test_recordMatch_emitsEvent` | `MatchRecorded` event emitted with correct fields |
| 3 | `test_recordMatch_storesEnvelopeHash` | `receipts[matchId]` == keccak256(abi.encode(envelope)) |
| 4 | `test_recordMatch_incrementsMatchCount` | `matchCount` increases by 1 |

### 3.2 Duplicate Prevention

| # | Test | Assertion |
|---|------|-----------|
| 5 | `test_recordMatch_revertsDuplicate` | Second call with same matchId reverts with `DuplicateMatch` |

### 3.3 Signature Verification

| # | Test | Assertion |
|---|------|-----------|
| 6 | `test_recordMatch_revertsUnauthorizedSigner` | Signature from non-authorized key reverts with `UnauthorizedSigner` |
| 7 | `test_recordMatch_revertsInvalidSignature` | Malformed signature reverts |
| 8 | `test_recordMatch_validSignatureFromAddedSigner` | New signer added → their signature accepted |
| 9 | `test_recordMatch_revokedSignerRejected` | Signer revoked → their signature rejected |

### 3.4 Batching

| # | Test | Assertion |
|---|------|-----------|
| 10 | `test_recordMatches_batchOf5` | 5 matches recorded, 5 events emitted, matchCount += 5 |
| 11 | `test_recordMatches_revertsEmptyBatch` | Empty array reverts with `EmptyBatch` |
| 12 | `test_recordMatches_revertsBatchTooLarge` | 201 items reverts with `BatchTooLarge` |
| 13 | `test_recordMatches_lengthMismatch` | Different array lengths reverts |
| 14 | `test_recordMatches_partialDuplicateReverts` | Batch with one duplicate → entire batch reverts |

### 3.5 Admin

| # | Test | Assertion |
|---|------|-----------|
| 15 | `test_setSigner_addsNewSigner` | `authorizedSigners[newAddr]` == true |
| 16 | `test_setSigner_revokesSigner` | `authorizedSigners[addr]` == false after revocation |
| 17 | `test_setSigner_onlyOwner` | Non-owner call reverts |
| 18 | `test_pause_stopsRecording` | `recordMatch()` reverts when paused |
| 19 | `test_unpause_resumesRecording` | `recordMatch()` succeeds after unpause |
| 20 | `test_pause_onlyOwner` | Non-owner cannot pause |

### 3.6 Verification

| # | Test | Assertion |
|---|------|-----------|
| 21 | `test_verifyEnvelope_validMatch` | Returns true for recorded envelope |
| 22 | `test_verifyEnvelope_unknownMatch` | Returns false for unknown matchId |
| 23 | `test_verifyEnvelope_tamperedData` | Returns false if any field modified |
| 24 | `test_isMatchRecorded_true` | Returns true after recording |
| 25 | `test_isMatchRecorded_false` | Returns false for unknown matchId |

### 3.7 Edge Cases

| # | Test | Assertion |
|---|------|-----------|
| 26 | `test_recordMatch_revertsZeroMatchId` | Zero matchId reverts |
| 27 | `test_ownerTransfer` | Ownable2Step transfer works correctly |

### 3.8 Gas Benchmarks

| # | Test | Output |
|---|------|--------|
| 28 | `test_gas_singleRecord` | Print gas used for 1 `recordMatch()` |
| 29 | `test_gas_batchOf25` | Print gas used for batch of 25 |
| 30 | `test_gas_batchOf100` | Print gas used for batch of 100 |

---

## 4. INTEGRATION TESTS

### 4.1 Rust ↔ Solidity Signature Compatibility

**Purpose:** Ensure that an EIP-712 signature generated in Rust (alloy) is accepted by the Solidity contract.

**Requires:** Base Sepolia RPC + deployed contract + funded signer.

Run: `cargo test --test integration -- --ignored` (ignored by default; run manually)

| # | Test | Steps |
|---|------|-------|
| 1 | `test_rust_signature_accepted_by_contract` | Sign envelope in Rust → submit to contract via alloy → no revert |
| 2 | `test_event_data_matches_envelope` | Submit → read event logs → all fields match input envelope |
| 3 | `test_verify_returns_true` | Submit → call `verifyEnvelope()` → returns true |
| 4 | `test_duplicate_rejected` | Submit twice with same matchId → second reverts |

### 4.2 Replay Export Integrity

Run: `cargo test -p ace-client -- replay`

| # | Test | Steps |
|---|------|-------|
| 1 | `test_replay_write_and_read` | Record frames → export → read file → deserialize → frames match |
| 2 | `test_replay_hash_matches_file` | Export → compute hash in code → compute hash with sha2 on file bytes → match |
| 3 | `test_empty_replay` | Match with 0 frames → still exports valid (degenerate but valid) |

---

## 5. END-TO-END TEST SCRIPT

**File:** `tests/e2e.sh`

This is a manual test script that exercises the full pipeline.

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== ACE CHAIN End-to-End Test ==="
echo ""

# Prerequisites check
echo "[1/8] Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found"; exit 1; }
command -v forge >/dev/null 2>&1 || { echo "ERROR: forge not found"; exit 1; }
command -v cast  >/dev/null 2>&1 || { echo "ERROR: cast not found"; exit 1; }
source .env 2>/dev/null || { echo "ERROR: .env not found"; exit 1; }
[ -n "${RECEIPTS_CONTRACT_ADDRESS:-}" ] || { echo "ERROR: RECEIPTS_CONTRACT_ADDRESS not set"; exit 1; }
[ -n "${SIGNER_PRIVATE_KEY:-}" ] || { echo "ERROR: SIGNER_PRIVATE_KEY not set"; exit 1; }
[ -n "${BASE_SEPOLIA_RPC_URL:-}" ] || { echo "ERROR: BASE_SEPOLIA_RPC_URL not set"; exit 1; }
echo "  ✓ All prerequisites met"

# Build
echo "[2/8] Building workspace..."
cargo build --workspace --release 2>&1 | tail -1
echo "  ✓ Build successful"

# Run Rust unit tests
echo "[3/8] Running Rust unit tests..."
cargo test --workspace 2>&1 | tail -3
echo "  ✓ Rust tests passed"

# Run Foundry tests
echo "[4/8] Running Solidity tests..."
cd contracts
forge test 2>&1 | tail -3
cd ..
echo "  ✓ Solidity tests passed"

# Check contract is deployed
echo "[5/8] Verifying contract on Base Sepolia..."
MATCH_COUNT=$(cast call "$RECEIPTS_CONTRACT_ADDRESS" "matchCount()" --rpc-url "$BASE_SEPOLIA_RPC_URL" 2>/dev/null)
echo "  Current match count: $MATCH_COUNT"
echo "  ✓ Contract accessible"

# Run match simulation and publish
echo "[6/8] Running match simulation + chain publish test..."
cargo test --test e2e_chain_test -- --ignored 2>&1 | tail -5
echo "  ✓ Match receipt published to Base Sepolia"

# Verify on-chain
echo "[7/8] Verifying receipt on-chain..."
# The e2e test should output the matchId; we check isMatchRecorded
echo "  (Verification done within the e2e test assertions)"
echo "  ✓ Receipt verified on-chain"

# Summary
echo "[8/8] Summary"
echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Rust unit tests:     PASSED"
echo "  Solidity tests:      PASSED"
echo "  Contract deployed:   YES"
echo "  Receipt published:   YES"
echo "  On-chain verified:   YES"
echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "=== ALL TESTS PASSED ==="
```

---

## 6. MANUAL PLAYTEST CHECKLIST

Run the game (`cargo run -p ace-client --release`) and verify:

### Menu Flow
- [ ] Main menu displays correctly
- [ ] "Play" navigates to hero select
- [ ] "Heroes" shows hero gallery
- [ ] "Quit" exits app

### Hero Select
- [ ] Both heroes shown with stat bars
- [ ] Stats visually different between heroes
- [ ] Difficulty selector works (Easy/Medium/Hard)
- [ ] "Start Match" begins match with selected settings

### Core Tennis
- [ ] Player moves with WASD, bounded by court
- [ ] Aim reticle follows mouse on opponent's court
- [ ] Reticle changes color near lines (green → yellow → red)
- [ ] Left mouse hold charges power (bar visible)
- [ ] Release fires ball toward reticle
- [ ] Ball physics: gravity, bounce, drag, spin
- [ ] Ball hits net → fault detected
- [ ] Ball lands out → out detected
- [ ] Scroll wheel cycles flat/topspin/slice
- [ ] Q = lob, E = drop shot
- [ ] Timing feedback displayed (PERFECT/GOOD/OK/LATE)

### Serve
- [ ] Spacebar triggers toss
- [ ] Power meter visible during toss
- [ ] Mouse aims at service box
- [ ] Click → serve executed
- [ ] Service box alternates correctly
- [ ] Double fault detection works

### AI
- [ ] AI moves to intercept ball
- [ ] AI returns shots to valid positions
- [ ] AI serves correctly
- [ ] Difficulty affects AI quality
- [ ] AI can win points (game isn't trivially easy on Hard)

### Scoring
- [ ] Score updates correctly after each point
- [ ] Games won displayed
- [ ] Server indicator correct
- [ ] Deuce/advantage handled
- [ ] Tiebreak at 6-6
- [ ] Set ends correctly

### Post-Match
- [ ] Final score displayed correctly
- [ ] Match stats shown (aces, winners, errors, etc.)
- [ ] "Export Replay" saves file, shows hash
- [ ] "Publish to Chain" submits transaction
- [ ] Transaction hash displayed
- [ ] Basescan link works
- [ ] "Play Again" returns to hero select
- [ ] "Main Menu" returns to menu

### Hero Stats
- [ ] Viktor: noticeably more powerful shots, slower movement
- [ ] Mika: noticeably faster, more precise, weaker shots
- [ ] Stamina depletes during play, affects performance
- [ ] Stamina recovers between points

### Performance
- [ ] Stable 60+ FPS
- [ ] No visible jitter or stuttering
- [ ] Ball physics smooth at all speeds

---

## 7. REGRESSION TEST TRIGGERS

Re-run the full test suite when:
- Any physics constant changes
- Scoring logic modified
- EIP-712 types modified (must match Rust AND Solidity)
- Contract redeployed
- Ball physics update logic changed
- Hero stats modified

**Critical cross-validation:** If `MatchEnvelope` struct changes in Solidity, it MUST be updated in Rust (`blockchain/types.rs`) and the EIP-712 typehash must be recalculated. Test T-051 and integration test 4.1 catch this.

---

*End of Test Plan — ACE CHAIN Prototype v0.1*
