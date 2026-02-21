# ACE CHAIN — Prototype Specification v0.1

## Vertical Slice: One Playable Match + One On-Chain Receipt on Base Sepolia

---

## 0. PROTOTYPE GOAL

**Deliver one complete loop:** select hero → play a tennis match → finish → see results → publish a signed receipt to Base Sepolia → verify receipt on-chain.

**Time budget:** 2–3 weeks for a single developer with Claude Code.

**Success criteria (all must pass):**

1. Player can start a match against AI, play points with mouse-driven aiming, and finish a full set.
2. Post-match screen shows score summary.
3. "Export Replay" produces a `.replay` binary file and displays its SHA-256 hash.
4. "Publish Receipt" sends one transaction to Base Sepolia and UI shows the tx hash + block explorer link.
5. The emitted `MatchRecorded` event on-chain contains the correct replay hash, score hash, and server signature that can be independently verified.

---

## 1. NON-GOALS (Cut Aggressively)

These are explicitly **out of scope** for v0.1:

| Cut | Rationale |
|-----|-----------|
| 12 heroes | Ship with **2 heroes** only (1 baseline brawler, 1 counter-puncher) |
| Special moves | **Zero specials** in v0.1. Plain tennis only. |
| Ranked mode | **Friendly only**. No matchmaking, no ELO/Glicko-2. |
| Multiplayer networking | **Local only**: player vs AI. No QUIC, no server-authoritative netcode. |
| Multiple courts/surfaces | **One hard court** only. |
| Cosmetics / skins | None. |
| Token / wallet linking | Not in prototype. Server EOA signs receipts. |
| Tournament mode | Not in prototype. |
| Audio | Minimal placeholder SFX only (ball hit, bounce). No music. |
| Chat / social | None. |
| Stamina system | Simplified: stamina depletes but no visual fatigue states. |
| Mobile | PC/Mac build only. |
| Replay viewer | Export file only; no in-game replay playback. |

---

## 2. TECHNOLOGY STACK (Prototype)

### 2.1 Game Client

| Component | Technology | Notes |
|-----------|-----------|-------|
| Language | **Rust** | |
| Engine | **Bevy 0.15+** | ECS, cross-platform rendering via wgpu |
| Physics | **bevy_rapier3d** | Ball physics, court collision |
| UI | **bevy_egui** | Menus, HUD, post-match screen |
| Audio | **bevy_kira_audio** | Placeholder SFX only |

### 2.2 Blockchain

| Component | Technology | Notes |
|-----------|-----------|-------|
| Network | **Base Sepolia** (testnet, chain ID 84532) | Move to Base Mainnet (8453) post-prototype |
| Contracts | **Solidity** via **Foundry** (forge/cast) | |
| Rust client | **alloy** | Transaction signing + submission from game |
| Signature | **EIP-712 typed data** + ECDSA (server EOA) | |
| Explorer | **Basescan Sepolia** | Link from post-match screen |

### 2.3 No Backend Services in v0.1

- No PostgreSQL, no Redis, no REST API, no matchmaking server.
- The game client runs a local authoritative simulation against AI.
- The game client holds the server signing key locally (acceptable for prototype on testnet).
- Post-match, the client directly submits the transaction to Base Sepolia via public RPC.

This will be refactored to a server-side signer in v0.2 when multiplayer is added.

### 2.4 Cargo.toml (Prototype Only)

```toml
[workspace]
members = ["crates/ace-client", "crates/ace-shared"]

[workspace.dependencies]
# Game engine
bevy = "0.15"
bevy_rapier3d = "0.28"
bevy_egui = "0.31"
bevy_kira_audio = "0.21"

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"

# Blockchain (alloy for EVM interaction)
alloy = { version = "0.9", features = ["full"] }

# Crypto
sha2 = "0.10"
k256 = "0.13"          # secp256k1 for ECDSA/EIP-712

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
rand = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
hex = "0.4"
```

---

## 3. GAME DESIGN (Prototype Subset)

### 3.1 Two Heroes (Fixed Stats, No Specials)

**Hero A — "Viktor" (Baseline Brawler)**

```
serve_power: 0.85    forehand_power: 0.90    speed: 0.60
serve_accuracy: 0.65  backhand_power: 0.75    acceleration: 0.55
volley_skill: 0.45    spin_control: 0.70      stamina: 0.80
reach: 0.75
```

**Hero B — "Mika" (Counter-Puncher)**

```
serve_power: 0.55    forehand_power: 0.60    speed: 0.95
serve_accuracy: 0.80  backhand_power: 0.70    acceleration: 0.90
volley_skill: 0.60    spin_control: 0.85      stamina: 0.90
reach: 0.55
```

Stats affect gameplay as defined in the full spec (Section 3.2 aiming precision, Section 3.4 ball physics modifiers). No special moves — `SpecialMove` enum and charge meter are not implemented in v0.1.

### 3.2 Mouse-Driven Aiming (Core Mechanic — Must Be Good)

This is the prototype's make-or-break feature. Implementation priority is highest.

**Input mapping:**

| Input | Action |
|-------|--------|
| WASD / Arrow Keys | Player movement on court |
| Mouse position | Aim reticle on opponent's court |
| Left mouse (hold + release) | Charge and fire shot |
| Scroll wheel | Cycle shot modifier: flat → topspin → slice |
| Q / E | Shot type: Q = lob, E = drop shot (default = groundstroke) |
| Spacebar | Serve toss |

**Aim reticle behavior:**

- A circle projected onto the opponent's court surface at the mouse cursor position.
- Circle **radius** = base precision × modifiers (distance from ball, hero stat, on-the-run penalty).
- The ball lands randomly within this circle (uniform distribution within the radius).
- Circle **color** fades green → yellow → red as target approaches out-of-bounds lines.
- Reticle is only visible when the player can hit (ball approaching their side).

**Shot charging:**

- Left mouse press begins charge. A small power bar appears near the player.
- Charge fills over 1.2 seconds to 100%.
- Holding past 1.3 seconds = overcharge → forced error (ball goes long/wide).
- Release fires the shot. Power determines ball speed.
- **Timing matters:** release should coincide with ball reaching the optimal hitting position. Early/late release degrades shot quality (wider aim circle, less power efficiency).

### 3.3 Serve Mechanic

1. Player presses SPACEBAR → ball toss animation begins.
2. A vertical power indicator appears beside the player.
3. Mouse cursor targets a spot inside the service box.
4. Left click at desired moment → serve executes.
5. Contact height (timing of click relative to toss apex) determines power vs. control trade-off.
6. Shot modifier (flat/slice/kick via scroll wheel) applies spin.

### 3.4 Ball Physics (Simplified for Prototype)

Server-authoritative physics runs at **120Hz** (fixed timestep in Bevy).

```rust
pub struct BallPhysicsParams {
    pub gravity: f32,              // -9.81
    pub air_drag: f32,             // 0.005
    pub magnus_coefficient: f32,   // 0.0008
    pub restitution: f32,          // 0.75 (hard court)
    pub ball_mass: f32,            // 0.057
    pub ball_radius: f32,          // 0.033
    pub max_speed: f32,            // 70.0 m/s
}
```

Per-tick update:
1. Apply gravity to velocity.y
2. Apply air drag (proportional to speed²)
3. Apply Magnus force (angular_velocity × velocity, scaled by magnus_coefficient)
4. Update position
5. Bounce detection: if position.y ≤ ball_radius, reflect velocity.y, apply restitution, apply surface friction to horizontal velocity
6. Net collision: if ball crosses net plane and position.y < net_height_at_x, it's a net fault
7. Out detection: if bounce position is outside court lines, point ends

**Determinism note (per feedback Section 2.1):** In v0.1, replays are **authoritative logs**, not deterministic input-only simulations. Each replay frame stores the server-computed ball state. The replay hash verifies the log integrity, not independent re-simulation.

### 3.5 Court Dimensions (Hard Court Only)

```
Court length:      23.77m
Singles width:      8.23m
Service box depth:  6.40m
Service box width:  4.115m (half-court)
Net height center:  0.914m
Net height posts:   1.067m
Baseline runoff:    6.0m
Side runoff:        3.66m
```

One game unit = one meter.

### 3.6 Scoring (Standard Tennis, One Set)

Prototype plays **one set** (first to 6 games, tiebreak at 6-6).

```rust
pub struct ScoreState {
    pub games: [u8; 2],
    pub points: GamePoints,
    pub server: usize,              // 0 or 1, alternates each game
    pub tiebreak: bool,
    pub set_complete: bool,
    pub winner: Option<usize>,
}

pub enum GamePoints {
    Regular { points: [u8; 2] },    // 0=0, 1=15, 2=30, 3=40, 4=AD
    Tiebreak { points: [u8; 2] },   // raw count, first to 7 with 2 ahead
}
```

Score transitions:
- 0 → 15 → 30 → 40 → Game (if opponent < 40)
- 40-40 = Deuce → AD → Game (or back to Deuce)
- Tiebreak at 6-6: first to 7 points, 2 clear; serve alternates every 2 points

### 3.7 AI Opponent (Minimal)

The AI needs to be good enough to rally, not good enough to be a product feature.

**Simple reactive AI:**
1. Move toward the ball's predicted landing position.
2. When in range, select a shot:
   - 70% groundstroke to a random valid target on the opponent's court
   - 15% cross-court angle
   - 10% down-the-line
   - 5% drop shot (if near net)
3. Charge time: random within "good" timing window (±0.12s of optimal).
4. AI difficulty scaled by: movement speed multiplier (0.6–1.0 of hero speed), aim precision multiplier (1.2–0.8 of hero base precision, higher = worse).

Three difficulty levels for prototype: Easy (0.6 speed, 1.2 precision), Medium (0.8, 1.0), Hard (1.0, 0.85).

### 3.8 Camera

**Behind-player** camera only in v0.1:
- 8m behind player, 5m above court
- Smooth tracking: lerp toward ball position with 0.3s lag
- Camera flips sides on serve change (player always at bottom of screen)

---

## 4. REPLAY FORMAT v0

Replays are **signed authoritative logs**. They are NOT independently re-simulable in v0.1.

```rust
#[derive(Serialize, Deserialize)]
pub struct ReplayV0 {
    pub version: u16,                    // 0
    pub match_id: [u8; 16],             // UUID bytes
    pub heroes: [u8; 2],                // Hero IDs
    pub surface: u8,                     // 0 = hard
    pub physics_params_hash: [u8; 32],  // SHA-256 of serialized BallPhysicsParams
    pub build_id: String,               // Cargo build hash / git commit
    pub determinism_mode: u8,           // 0 = AuthoritativeLog
    pub started_at: i64,                // Unix timestamp
    pub frames: Vec<ReplayFrame>,
    pub final_score: ScoreState,
    pub stats: [MatchStats; 2],
}

#[derive(Serialize, Deserialize)]
pub struct ReplayFrame {
    pub tick: u64,
    pub inputs: [PlayerInput; 2],       // Both player and AI inputs
    pub ball: BallSnapshot,             // Authoritative ball state
    pub players: [PlayerSnapshot; 2],   // Authoritative player positions
    pub events: Vec<GameEvent>,         // Point scored, fault, etc.
}

#[derive(Serialize, Deserialize)]
pub struct MatchStats {
    pub aces: u16,
    pub double_faults: u16,
    pub winners: u16,
    pub unforced_errors: u16,
    pub first_serve_pct: f32,
    pub points_won: u16,
    pub longest_rally: u16,
}
```

**Export flow:**
1. Match ends → serialize `ReplayV0` to bincode → write to `replays/{match_id}.replay`
2. Compute `replay_hash = SHA-256(replay_bytes)`
3. Display hash in post-match screen

---

## 5. BLOCKCHAIN INTEGRATION (Base Sepolia)

See **SPEC-CHAIN-v0.1-BASE.md** for the full contract specification.

### 5.1 Prototype On-Chain Flow

```
Match ends
    │
    ▼
Client computes:
  • resultHash = SHA-256(canonical_result_json)
  • replayHash = SHA-256(replay_binary)
  • Constructs MatchEnvelope struct
  • Signs envelope with EIP-712 typed data (local server key)
    │
    ▼
Client calls AceChainReceipts.recordMatch(envelope, signature)
  on Base Sepolia via alloy
    │
    ▼
Contract:
  • Verifies ECDSA signature against authorized signer
  • Checks matchId not already recorded
  • Stores matchId → envelopeHash
  • Emits MatchRecorded event with full envelope data
    │
    ▼
Client receives tx hash
  • Displays tx hash in post-match screen
  • Provides link: https://sepolia.basescan.org/tx/{txHash}
```

### 5.2 Prototype Signing (Local Key)

For v0.1 only: the game client holds a local Ethereum private key in a `.env` file. This key is the "server signer" for testnet.

```env
# .env (prototype only — NEVER in production)
SIGNER_PRIVATE_KEY=0xabc...
BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
RECEIPTS_CONTRACT_ADDRESS=0x...
```

**In production (v0.2+):** the signing key moves to a server-side KMS/HSM. The game client sends match results to the server API, which signs and submits. See SPEC-CHAIN Section 6 (Signer Governance).

### 5.3 Gas Funding (Testnet)

- Use Base Sepolia faucet to fund the signer address.
- Prototype submits one transaction per match (no batching in v0.1).
- Batching (`recordMatches()`) is implemented in the contract but not used by the client until v0.2.

---

## 6. PROJECT STRUCTURE (Prototype)

```
ace-chain/
├── Cargo.toml                       # Workspace
├── .env                             # Signer key + RPC URL (gitignored)
├── .gitignore
├── README.md
├── SPEC-PROTOTYPE-v0.1.md           # This file
├── SPEC-CHAIN-v0.1-BASE.md          # Contract spec
├── TASKS.md                         # Claude Code task breakdown
├── TESTPLAN.md                      # Test plan
│
├── crates/
│   ├── ace-client/                  # Bevy game client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs              # Bevy app setup, state machine
│   │       ├── states/
│   │       │   ├── mod.rs
│   │       │   ├── menu.rs          # Main menu (Play / Heroes / Quit)
│   │       │   ├── hero_select.rs   # Pick hero + difficulty
│   │       │   ├── playing.rs       # In-match state manager
│   │       │   └── post_match.rs    # Score + replay export + publish
│   │       ├── systems/
│   │       │   ├── mod.rs
│   │       │   ├── input.rs         # Keyboard + mouse input capture
│   │       │   ├── aiming.rs        # Aim reticle projection + precision
│   │       │   ├── shot.rs          # Charge, timing, shot execution
│   │       │   ├── serve.rs         # Serve toss + power + placement
│   │       │   ├── movement.rs      # Player WASD movement + bounds
│   │       │   ├── ball_physics.rs  # 120Hz ball simulation
│   │       │   ├── scoring.rs       # Point/game/set state machine
│   │       │   ├── ai.rs            # AI opponent logic
│   │       │   ├── camera.rs        # Behind-player camera
│   │       │   ├── hud.rs           # Score display, power bar, reticle
│   │       │   └── stamina.rs       # Stamina depletion/regen
│   │       ├── resources/
│   │       │   ├── mod.rs
│   │       │   ├── heroes.rs        # Hero stat definitions
│   │       │   ├── court.rs         # Court dimensions + mesh generation
│   │       │   └── physics.rs       # Physics constants
│   │       ├── blockchain/
│   │       │   ├── mod.rs
│   │       │   ├── signer.rs        # EIP-712 signing (alloy)
│   │       │   ├── submitter.rs     # Transaction submission to Base
│   │       │   └── types.rs         # MatchEnvelope, on-chain types
│   │       └── replay/
│   │           ├── mod.rs
│   │           ├── recorder.rs      # Frame-by-frame recording during match
│   │           ├── exporter.rs      # Serialize + hash + write file
│   │           └── types.rs         # ReplayV0 struct
│   │
│   └── ace-shared/                  # Shared types (for future server crate)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── types.rs             # HeroStats, ShotType, GameEvent, etc.
│           ├── scoring.rs           # Tennis scoring state machine (pure logic)
│           ├── physics.rs           # BallPhysicsParams, constants
│           └── protocol.rs          # MatchEnvelope, result types
│
├── contracts/                       # Foundry project
│   ├── foundry.toml
│   ├── src/
│   │   └── AceChainReceipts.sol     # Main contract
│   ├── test/
│   │   └── AceChainReceipts.t.sol   # Forge tests
│   └── script/
│       └── Deploy.s.sol             # Deployment script
│
├── replays/                         # Local replay storage (gitignored)
│
└── assets/
    ├── models/                      # Placeholder 3D assets
    │   ├── court.glb               # Hard court model
    │   ├── player.glb              # Generic player model
    │   ├── ball.glb                # Tennis ball
    │   └── net.glb                 # Net
    ├── textures/
    │   ├── court_hard.png
    │   └── ball.png
    └── audio/
        ├── hit.ogg                 # Ball hit placeholder
        └── bounce.ogg              # Ball bounce placeholder
```

---

## 7. UI FLOW (Prototype)

```
┌──────────────┐
│  MAIN MENU   │
│              │
│  ► Play      │──┐
│  ► Heroes    │──┼──► Hero Gallery (stat cards for 2 heroes)
│  ► Quit      │  │
└──────────────┘  │
                  ▼
         ┌─────────────────┐
         │  HERO SELECT    │
         │                 │
         │  Choose hero    │
         │  Choose AI diff │ (Easy / Medium / Hard)
         │  [Start Match]  │
         └────────┬────────┘
                  ▼
         ┌─────────────────┐
         │  PLAYING        │
         │                 │
         │  Tennis match    │
         │  HUD: score,    │
         │  power bar,     │
         │  stamina bar,   │
         │  shot type      │
         └────────┬────────┘
                  ▼ (set complete)
         ┌─────────────────┐
         │  POST-MATCH     │
         │                 │
         │  Final score    │
         │  Match stats    │
         │  [Export Replay] │──► saves .replay file, shows SHA-256
         │  [Publish Chain] │──► submits tx, shows tx hash + basescan link
         │  [Play Again]   │──► back to Hero Select
         │  [Main Menu]    │──► back to Main Menu
         └─────────────────┘
```

---

## 8. ACCEPTANCE CRITERIA (Detailed)

### AC-1: Core Tennis Loop
- [ ] Player avatar moves on court with WASD, bounded by court + runoff area.
- [ ] Mouse cursor projects aim reticle on opponent's court surface.
- [ ] Reticle size varies with distance-to-ball and hero stats.
- [ ] Left mouse hold charges shot; power bar visible.
- [ ] Release fires ball toward reticle position (with precision scatter).
- [ ] Ball follows physics: gravity, drag, Magnus force, bounce.
- [ ] Net collision stops ball; out-of-bounds detected correctly.
- [ ] Scroll wheel cycles flat/topspin/slice (visible indicator in HUD).
- [ ] Q = lob, E = drop shot, default = groundstroke.
- [ ] Serve works: spacebar toss → mouse aim → click to serve.

### AC-2: Scoring
- [ ] Points: 0-15-30-40-Game, with deuce/advantage.
- [ ] Games: first to 6, tiebreak at 6-6.
- [ ] Tiebreak: first to 7, 2 clear, serve alternates every 2 points.
- [ ] Server changes each game.
- [ ] HUD shows current score at all times.

### AC-3: AI Opponent
- [ ] AI moves toward predicted ball landing.
- [ ] AI returns shots to valid court positions.
- [ ] AI difficulty affects speed and precision.
- [ ] AI can serve.

### AC-4: Replay Export
- [ ] After match, "Export Replay" button writes `replays/{match_id}.replay`.
- [ ] File is a valid bincode-serialized `ReplayV0`.
- [ ] SHA-256 hash displayed in UI matches actual file hash.

### AC-5: On-Chain Receipt
- [ ] After match, "Publish to Chain" button submits transaction to Base Sepolia.
- [ ] Transaction calls `recordMatch()` on deployed `AceChainReceipts` contract.
- [ ] Transaction includes valid EIP-712 signature from signer key.
- [ ] Contract emits `MatchRecorded` event with correct data.
- [ ] UI shows transaction hash.
- [ ] Clicking tx hash opens `https://sepolia.basescan.org/tx/{hash}`.
- [ ] Calling the contract again with the same matchId reverts (duplicate protection).

### AC-6: Hero Stats Affect Gameplay
- [ ] Viktor hits harder but moves slower than Mika.
- [ ] Mika's aim reticle is smaller (more precise) than Viktor's on equivalent shots.
- [ ] Stat differences are perceptible within 2-3 rallies of play.

---

## 9. KNOWN LIMITATIONS & FUTURE WORK

| Limitation in v0.1 | Resolution in v0.2+ |
|---------------------|---------------------|
| Local signer key | Server-side KMS/HSM signer |
| No multiplayer | QUIC networking + server-authoritative |
| No batching on-chain | `recordMatches()` batch call from server |
| One court surface | Add clay + grass with physics modifiers |
| 2 heroes | Expand to 8–12 with balanced specials |
| No ranked | Glicko-2 ratings, matchmaking queue |
| No replay viewer | In-game replay playback |
| No spectator mode | Spectator camera + live streaming |
| No key remapping | Settings screen with full rebinding |
| No controller support | Gamepad input mapping |

---

*End of Prototype Specification v0.1*
