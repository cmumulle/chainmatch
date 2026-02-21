# ACE CHAIN — Claude Code Task Breakdown

## Execution Plan: Prototype v0.1

---

## How to Use This File

Each task is a self-contained ticket designed for Claude Code execution. Work through them **in order** — later tasks depend on earlier ones.

**Format per task:**
- **Files:** What to create or modify
- **Done when:** Explicit acceptance criteria
- **Test:** Command to verify completion

**Estimated effort:** 0.5–2 hours per task. Total: ~60–80 hours.

---

## PHASE 0: Project Scaffold (Tasks 1–6)

### T-001: Initialize Cargo workspace

**Files:** `Cargo.toml`, `crates/ace-shared/Cargo.toml`, `crates/ace-shared/src/lib.rs`, `crates/ace-client/Cargo.toml`, `crates/ace-client/src/main.rs`, `.gitignore`

**Done when:**
- Cargo workspace compiles with both crates.
- `ace-client` has an empty Bevy app that opens a window with a dark background.
- `.gitignore` excludes `target/`, `.env`, `replays/`, `contracts/out/`, `contracts/cache/`.

**Test:** `cargo build --workspace && cargo run -p ace-client` → window opens, no errors.

---

### T-002: Initialize Foundry project

**Files:** `contracts/foundry.toml`, `contracts/remappings.txt`, `contracts/src/.gitkeep`, `contracts/test/.gitkeep`, `contracts/script/.gitkeep`

**Done when:**
- `forge build` succeeds in `contracts/` directory.
- OpenZeppelin contracts installed as dependency.
- `remappings.txt` maps `@openzeppelin/contracts/` correctly.

**Test:** `cd contracts && forge install OpenZeppelin/openzeppelin-contracts --no-commit && forge build`

---

### T-003: Create .env template

**Files:** `.env.example`, `.env` (gitignored)

**Done when:**
- `.env.example` has all required keys with placeholder values:
  ```
  SIGNER_PRIVATE_KEY=0x...
  BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
  RECEIPTS_CONTRACT_ADDRESS=0x...
  DEPLOYER_PRIVATE_KEY=0x...
  OWNER_ADDRESS=0x...
  SIGNER_ADDRESS=0x...
  ```
- `.env` is in `.gitignore`.

**Test:** File exists, all keys present.

---

### T-004: Create directory structure

**Files:** Create all directories matching SPEC-PROTOTYPE Section 6 project structure. Add `.gitkeep` to empty dirs.

**Done when:** `find . -name .gitkeep` lists all expected directories: `assets/models/`, `assets/textures/`, `assets/audio/`, `replays/`, `crates/ace-client/src/states/`, `crates/ace-client/src/systems/`, `crates/ace-client/src/resources/`, `crates/ace-client/src/blockchain/`, `crates/ace-client/src/replay/`.

**Test:** `tree -d crates/` matches expected structure.

---

### T-005: Define ace-shared types

**Files:** `crates/ace-shared/src/lib.rs`, `crates/ace-shared/src/types.rs`

**Done when:** The following types are defined with `Serialize`, `Deserialize`, `Clone`, `Debug`:
- `HeroId` (u8 newtype)
- `Archetype` (enum: BaselineBrawler, ServeAndVolley, CounterPuncher, AllRounder)
- `HeroStats` (all 10 stat fields + name + id + archetype)
- `ShotType` (enum: Flat, Topspin, Slice, Lob, DropShot, Smash)
- `ShotModifier` (enum: Flat, Topspin, Slice)
- `CourtSurface` (enum with physics params: Hard, Clay, Grass)
- `MatchType` (enum: Friendly, Ranked, Tournament)
- `PlayerId` (wrapper around `[u8; 32]`)

**Test:** `cargo build -p ace-shared` compiles. `cargo test -p ace-shared` → types can be serialized/deserialized roundtrip.

---

### T-006: Define ace-shared scoring logic

**Files:** `crates/ace-shared/src/scoring.rs`

**Done when:**
- `ScoreState` struct with: `games: [u8; 2]`, `points: GamePoints`, `server: usize`, `tiebreak: bool`, `set_complete: bool`, `winner: Option<usize>`.
- `GamePoints` enum: `Regular { points: [u8; 2] }`, `Tiebreak { points: [u8; 2] }`.
- `ScoreState::new()` → fresh state, server = 0.
- `ScoreState::point_won(player: usize)` → advances score correctly.
- Handles: 0/15/30/40/Game, Deuce/Advantage, tiebreak at 6-6, tiebreak scoring (first to 7, 2 clear), server alternation, set completion.
- `ScoreState::display_points()` → returns human-readable score string (e.g., "30-15").
- `ScoreState::display_games()` → returns "6-4" style string.

**Test:** `cargo test -p ace-shared -- scoring` — at least 15 unit tests covering:
1. Normal game progression (0-15-30-40-Game)
2. Deuce and advantage
3. Advantage lost → back to deuce
4. Game won from advantage
5. Set won at 6-0
6. Set won at 6-4
7. Tiebreak triggered at 6-6
8. Tiebreak won at 7-5
9. Tiebreak extended (7-7 → 8-6 etc)
10. Server alternation each game
11. Tiebreak server alternation (every 2 points)
12. Set complete flag set correctly
13. Winner set correctly
14. Multiple games in sequence
15. Full set simulation (randomized, check invariants)

---

## PHASE 1: Court & Rendering (Tasks 7–12)

### T-007: Bevy app state machine

**Files:** `crates/ace-client/src/main.rs`, `crates/ace-client/src/states/mod.rs`, `crates/ace-client/src/states/menu.rs`, `crates/ace-client/src/states/hero_select.rs`, `crates/ace-client/src/states/playing.rs`, `crates/ace-client/src/states/post_match.rs`

**Done when:**
- Bevy `States` enum: `Menu`, `HeroSelect`, `Playing`, `PostMatch`.
- App starts in `Menu` state.
- Each state module has placeholder `OnEnter`/`OnExit` systems that log to console.
- State transitions work: Menu → HeroSelect → Playing → PostMatch → Menu.

**Test:** `cargo run -p ace-client` → starts in Menu. Press Enter cycles through states (temporary debug input). Console logs state changes.

---

### T-008: Render hard court

**Files:** `crates/ace-client/src/resources/court.rs`, `crates/ace-client/src/systems/mod.rs`

**Done when:**
- A 3D hard court is rendered with correct dimensions (23.77m × 8.23m singles).
- Court has: baseline, service lines, center service line, singles sidelines, net (thin box at correct height).
- Court surface is a textured plane (solid blue/green color acceptable for prototype).
- Run-off area rendered in a slightly different shade.
- Net rendered as a thin rectangular prism (0.914m center, 1.067m posts).
- Camera positioned behind-and-above looking down-court.

**Test:** `cargo run -p ace-client` → visually inspect court. Lines visible, net visible, proportions correct.

---

### T-009: Spawn player entity

**Files:** `crates/ace-client/src/systems/movement.rs`, update `playing.rs`

**Done when:**
- Player entity spawned as a colored capsule/box at baseline center.
- WASD moves the player on the court (XZ plane).
- Movement bounded by court + runoff area.
- Movement speed = 8.0 m/s base (will be modified by hero stats later).
- Acceleration and deceleration applied (not instant stop/start).

**Test:** `cargo run -p ace-client` → enter Playing state → WASD moves player → player cannot leave court area.

---

### T-010: Spawn ball entity

**Files:** `crates/ace-client/src/systems/ball_physics.rs`, `crates/ace-shared/src/physics.rs`

**Done when:**
- `BallPhysicsParams` defined in ace-shared with all constants from spec.
- Ball entity spawned as a yellow sphere (radius 0.033m, but visually scaled up 3× for visibility).
- Ball physics system runs at fixed 120Hz timestep.
- Ball responds to gravity (falls to court).
- Ball bounces on court surface (restitution applied).
- Ball stops when velocity < threshold.
- For this task only: ball spawned at (0, 3, 0) and dropped to test bounce.

**Test:** `cargo run -p ace-client` → ball drops, bounces 3-4 times, settles on court. Physics looks natural.

---

### T-011: Ball air drag and Magnus force

**Files:** Update `crates/ace-client/src/systems/ball_physics.rs`

**Done when:**
- Air drag applied (proportional to speed²).
- Magnus force applied (angular_velocity × velocity × coefficient).
- Ball can be given initial velocity + spin and follows a realistic curved trajectory.
- Add debug system: press F1 to launch ball with preset velocity/spin combinations to test trajectories.

**Test:** `cargo run -p ace-client` → F1 launches ball → ball curves visibly with topspin (dips), slices (floats), and flat (straight). Ball decelerates over time.

---

### T-012: Net and out-of-bounds detection

**Files:** Update `crates/ace-client/src/systems/ball_physics.rs`

**Done when:**
- Ball hitting the net (crossing net plane with y < net_height_at_x) triggers a `NetFault` event.
- Ball bouncing outside court lines triggers an `OutOfBounds` event.
- Ball bouncing inside court lines triggers a `ValidBounce` event.
- Events include: bounce position, which side of court.
- Debug: console logs events when triggered.

**Test:** F1 launch presets that (a) hit net, (b) land in, (c) land out. Console shows correct events for each.

---

## PHASE 2: Mouse Aiming & Shots (Tasks 13–22)

### T-013: Mouse ray-to-court projection

**Files:** `crates/ace-client/src/systems/aiming.rs`

**Done when:**
- Mouse position projected as a ray from camera through screen point.
- Ray intersected with court plane (y=0) to get court coordinates.
- A small debug sphere rendered at the intersection point.
- Intersection only valid when mouse is over the opponent's court half.

**Test:** Move mouse → debug sphere follows on court surface. Only appears on opponent's side.

---

### T-014: Aim reticle with precision circle

**Files:** Update `crates/ace-client/src/systems/aiming.rs`, `crates/ace-client/src/systems/hud.rs`

**Done when:**
- Debug sphere replaced with a projected circle (ring) on the court surface.
- Circle radius = base precision value (start with fixed 0.5m, hero-stat-driven later).
- Circle color: green when center is >1m from lines, yellow when <1m, red when <0.3m or outside lines.
- Reticle only visible during `CanHit` state (placeholder: always visible for now).

**Test:** Move mouse around court → reticle follows, changes color near lines and out-of-bounds areas.

---

### T-015: Shot charging system

**Files:** `crates/ace-client/src/systems/shot.rs`, `crates/ace-client/src/systems/hud.rs`

**Done when:**
- Left mouse press begins charge timer.
- Power bar UI element appears near player (egui overlay or 3D billboard).
- Power fills from 0% to 100% over 1.2 seconds.
- Past 1.3 seconds, bar turns red (overcharge).
- Left mouse release records final charge percentage and charge duration.
- `ShotCharged` event emitted with: power (0.0–1.0+), charge_duration, timestamp.

**Test:** Hold left mouse → power bar fills → release → console logs power percentage. Overcharge turns bar red.

---

### T-016: Shot execution (ball launch from player)

**Files:** Update `crates/ace-client/src/systems/shot.rs`, `crates/ace-client/src/systems/ball_physics.rs`

**Done when:**
- When `ShotCharged` event fires AND ball is near player (within hitting range):
  - Compute launch direction from player toward aim reticle position.
  - Compute launch speed from charge power × hero's power stat.
  - Compute launch angle to clear the net and land near target (basic trajectory calculation).
  - Apply precision scatter: actual landing = target + random offset within reticle radius.
  - Set ball velocity and angular_velocity based on shot modifier (flat/topspin/slice).
- Ball launches toward opponent's court.

**Test:** Stand near ball → aim at opponent's court → hold and release left mouse → ball flies toward target with visible arc. Multiple shots land in different spots within the reticle circle.

---

### T-017: Shot modifier cycling (flat/topspin/slice)

**Files:** `crates/ace-client/src/systems/input.rs`, update `shot.rs`, update `hud.rs`

**Done when:**
- Scroll wheel cycles through: Flat → Topspin → Slice → Flat.
- Current modifier displayed in HUD (text label + icon).
- Modifier affects ball physics:
  - Flat: no spin, highest speed.
  - Topspin: forward spin (angular_velocity), ball dips faster, bounces higher.
  - Slice: backspin, ball floats more, bounces lower and skids.
- Spin values scaled by hero's `spin_control` stat.

**Test:** Hit balls with each modifier → topspin visibly dips and bounces high, slice floats and bounces low, flat goes straight. Visual difference clear.

---

### T-018: Shot types (lob, drop shot)

**Files:** Update `crates/ace-client/src/systems/shot.rs`, update `input.rs`

**Done when:**
- Q key → Lob mode: high arc, lands deep in court. Lower power, higher angle.
- E key → Drop Shot mode: soft touch, lands near net. Very low power, high precision needed.
- Default (no Q/E) → Groundstroke: standard trajectory.
- Shot type indicator in HUD.
- Lob and drop shot modify the launch angle calculation and power scaling.

**Test:** Hit lobs → ball goes high and deep. Hit drop shots → ball barely clears net and dies. Both land in valid court area.

---

### T-019: Smash detection and execution

**Files:** Update `crates/ace-client/src/systems/shot.rs`

**Done when:**
- When ball is above player at height > 2.5m (overhead position), shot type auto-switches to Smash.
- Smash: very high power, steep downward angle, low precision (large reticle).
- HUD shows "SMASH" indicator when available.
- Smash cannot be executed on low balls (only when ball position.y > threshold near player).

**Test:** Hit a high lob to the player → "SMASH" appears → hit → ball slams down steeply with high speed.

---

### T-020: Serve mechanic — toss and power

**Files:** `crates/ace-client/src/systems/serve.rs`, update `input.rs`, update `hud.rs`

**Done when:**
- During serve state: player positioned behind baseline.
- Spacebar triggers ball toss (ball goes up with initial velocity).
- A vertical power meter appears.
- Ball rises to apex then falls.
- Mouse aims at service box (reticle only within the correct service box).
- Left click during ball descent → serve executed.
- Contact height determines power/control: higher = more power, lower = more control (smaller reticle).
- If ball falls below knee height without clicking → fault (ball not hit).

**Test:** Press space → ball tosses → click at apex → powerful serve. Click late → weaker serve. Miss entirely → fault.

---

### T-021: Serve placement and spin

**Files:** Update `crates/ace-client/src/systems/serve.rs`

**Done when:**
- Serve target constrained to correct service box (alternates deuce/ad side).
- Scroll wheel selects serve type: flat / slice / kick.
- Flat serve: fastest, least curve.
- Slice serve: curves away from receiver (for right-handed server).
- Kick serve: high bounce, kicks toward receiver's backhand.
- Double fault detection: two consecutive faults → point to receiver.
- Second serve auto-triggered after first fault.

**Test:** Serve to deuce box → ad box (alternating). Slice visibly curves. Two faults = double fault logged.

---

### T-022: Timing window and shot quality

**Files:** Update `crates/ace-client/src/systems/shot.rs`, update `aiming.rs`

**Done when:**
- Shot quality depends on timing: when the player releases the mouse relative to the ball reaching the optimal hit position.
- Perfect (±0.05s): reticle at minimum size, full power efficiency.
- Good (±0.12s): reticle slightly larger, 90% power.
- OK (±0.20s): reticle larger, 75% power.
- Late/Early (±0.35s): reticle much larger, 50% power.
- Outside window: miss / shanked shot (random direction, very weak).
- HUD shows timing feedback: "PERFECT!" / "GOOD" / "OK" / "LATE" flash text.

**Test:** Hit balls at different timings → "PERFECT!" shots are accurate, "LATE" shots scatter widely. Visual feedback appears.

---

## PHASE 3: AI & Match Loop (Tasks 23–32)

### T-023: Basic AI movement

**Files:** `crates/ace-client/src/systems/ai.rs`

**Done when:**
- AI opponent entity spawned on opposite side of court.
- AI predicts ball landing position (simple forward projection of ball trajectory).
- AI moves toward predicted position at speed × difficulty multiplier.
- AI stops when close to predicted position.
- AI returns to center court between rallies.

**Test:** Launch ball toward AI side → AI moves to intercept. AI doesn't leave court boundaries.

---

### T-024: AI shot execution

**Files:** Update `crates/ace-client/src/systems/ai.rs`

**Done when:**
- When ball is within AI's hitting range, AI executes a return shot.
- AI selects target: 70% center, 15% cross-court, 10% down-the-line, 5% drop shot.
- AI charge timing: random within "good" window (±0.12s).
- AI aim precision: base precision × difficulty multiplier (1.2 for Easy, 1.0 for Medium, 0.85 for Hard).
- AI selects shot modifier randomly (70% flat, 20% topspin, 10% slice).

**Test:** Rally with AI → AI returns shots to varied positions. Difficulty affects AI's accuracy and speed.

---

### T-025: AI serving

**Files:** Update `crates/ace-client/src/systems/ai.rs`, update `serve.rs`

**Done when:**
- When AI is serving: AI performs serve after 2-second delay.
- AI serve targets random position within service box.
- AI serve power: medium (0.6–0.8 × max).
- AI occasionally double faults (5% chance per serve for Medium difficulty).

**Test:** AI serves → ball lands in service box most of the time. Occasional faults.

---

### T-026: Point flow state machine

**Files:** `crates/ace-client/src/states/playing.rs` (major update)

**Done when:**
- Point states: `Serving` → `Rally` → `PointOver` → `NextPoint` → `Serving`.
- `Serving`: server positioned, serve mechanic active.
- `Rally`: ball in play, both players hit.
- `PointOver`: triggered by: ball bounces twice on one side, ball into net, ball out after bounce, double fault, ace.
- `NextPoint`: brief pause (1.5s), update score, determine next server, reset positions.
- Point winner determined correctly for all end conditions.
- Players return to baseline positions between points.

**Test:** Play a full point through all stages. Score updates correctly. Players reset positions.

---

### T-027: Integrate scoring with match flow

**Files:** Update `playing.rs`, integrate `ace-shared/scoring.rs`

**Done when:**
- `ScoreState` from ace-shared drives match flow.
- After each point, `point_won()` called.
- Game changes (new game) trigger server switch.
- Tiebreak at 6-6 changes serve rotation.
- Set completion triggers transition to `PostMatch` state.
- HUD shows live score: "Games: 3-2 | Points: 30-15 | Serving: Player".

**Test:** Play an entire set against AI. Score progresses correctly. Tiebreak works. Set ends properly.

---

### T-028: Hero stat integration

**Files:** `crates/ace-client/src/resources/heroes.rs`, update `movement.rs`, `shot.rs`, `aiming.rs`, `serve.rs`, `ai.rs`

**Done when:**
- Viktor and Mika stats loaded from `heroes.rs` constants.
- Player movement speed scaled by `hero.speed` and `hero.acceleration`.
- Shot power scaled by `hero.forehand_power` or `hero.backhand_power` (determine by ball position relative to player).
- Aim reticle base radius scaled inversely by relevant power stat.
- Serve power scaled by `hero.serve_power`, serve reticle by `hero.serve_accuracy`.
- AI uses opponent hero's stats for its calculations.

**Test:** Play as Viktor vs Mika AI: Viktor hits harder but is slower. Play as Mika: faster, more precise, less power. Difference noticeable.

---

### T-029: Stamina system (simplified)

**Files:** `crates/ace-client/src/systems/stamina.rs`, update `hud.rs`

**Done when:**
- Stamina bar in HUD (100% at match start).
- Stamina depletes: per meter moved (0.3/m), per shot (1.0), per sprint (holding shift = 1.5× speed but 0.8/s extra drain).
- Stamina regenerates: 2.0/s when idle, 0.5/s when walking, 5.0 flat per point transition.
- Below 50%: aim reticle 20% larger.
- Below 25%: movement 15% slower, reticle 40% larger.
- Below 10%: movement 30% slower, reticle 60% larger.
- Stamina scaled by hero's `stamina` stat (max = 100 × stamina_stat, regen also scaled).

**Test:** Run around court → stamina drops → reticle grows → movement slows. Stand still → stamina recovers. Between points → chunk recovery.

---

### T-030: Match stats tracking

**Files:** `crates/ace-client/src/replay/types.rs` (MatchStats), update `playing.rs`

**Done when:**
- `MatchStats` struct tracks per-player: aces, double_faults, winners, unforced_errors, first_serve_pct, points_won, longest_rally.
- Stats updated in real-time during match.
- Ace: serve that isn't touched by receiver.
- Winner: shot that opponent couldn't reach.
- Unforced error: shot into net or out when opponent's previous shot was not a "forcing" shot (simplify: if ball speed < 20m/s on the incoming shot, error is unforced).
- Rally counter incremented each hit, longest tracked.

**Test:** Play a set → post-match stats are reasonable (aces > 0 for powerful hero, errors tracked).

---

### T-031: Camera system

**Files:** `crates/ace-client/src/systems/camera.rs`

**Done when:**
- Camera behind player: 8m back, 5m up from player position.
- Camera looks toward the net (slightly ahead of player toward ball).
- Smooth interpolation: camera lerps toward target position with 0.3s lag.
- Camera flips 180° on side change (after serve change) with smooth transition (0.5s).
- Ball tracking: camera look-at point subtly shifts toward ball position (20% weight).

**Test:** Play a set → camera stays behind player → flips at side changes smoothly → ball is always visible.

---

### T-032: HUD layout (egui)

**Files:** `crates/ace-client/src/systems/hud.rs` (consolidate all HUD elements)

**Done when:** Single egui overlay showing:
- Score: top-center, large font. "Games: 3-2 | 30-15".
- Server indicator: small dot next to serving player's score.
- Power bar: near player (bottom-center), visible only when charging.
- Stamina bar: bottom-left, always visible.
- Shot type/modifier: bottom-right ("Groundstroke | Topspin").
- Timing feedback: center-screen flash ("PERFECT!" etc.), fades after 0.5s.
- All elements have transparent backgrounds, don't obscure gameplay.

**Test:** Visual inspection during play. All elements visible, readable, non-intrusive.

---

## PHASE 4: Menu & Hero Select (Tasks 33–37)

### T-033: Main menu screen

**Files:** `crates/ace-client/src/states/menu.rs`

**Done when:**
- Menu shows: "ACE CHAIN" title, buttons: "Play", "Heroes", "Quit".
- "Play" → transitions to HeroSelect.
- "Heroes" → transitions to a hero gallery (placeholder for now, just show stats).
- "Quit" → exits app.
- Styled with egui: dark background, centered layout, clear font.

**Test:** `cargo run -p ace-client` → menu appears → buttons work → state transitions correct.

---

### T-034: Hero select screen

**Files:** `crates/ace-client/src/states/hero_select.rs`

**Done when:**
- Shows Viktor and Mika side by side with stat bars.
- Each stat (10 stats) shown as a labeled horizontal bar (0.0–1.0 scale).
- Player clicks to select hero (highlighted border).
- Difficulty dropdown: Easy / Medium / Hard.
- "Start Match" button → transitions to Playing with selected hero + difficulty.
- Selected hero and AI hero stored as resources for the Playing state.

**Test:** Select each hero → verify stat bars differ → start match → correct hero stats loaded.

---

### T-035: Post-match results screen

**Files:** `crates/ace-client/src/states/post_match.rs`

**Done when:**
- Shows: "Match Complete" header.
- Final score displayed prominently (e.g., "Viktor 6 - 4 Mika").
- Match stats table for both players (from MatchStats).
- Buttons: "Export Replay", "Publish to Chain", "Play Again", "Main Menu".
- "Play Again" → HeroSelect.
- "Main Menu" → Menu.
- "Export Replay" and "Publish to Chain" are placeholder buttons (wired in later tasks).

**Test:** Complete a match → post-match screen shows correct score and stats → buttons navigate correctly.

---

### T-036: Hero gallery screen

**Files:** `crates/ace-client/src/states/menu.rs` (or new `hero_gallery.rs`)

**Done when:**
- Shows both heroes with full stat breakdown.
- Each hero: name, archetype label, 10 stat bars, short description.
- "Back" button returns to Menu.

**Test:** Navigate to gallery → both heroes displayed → stats accurate → back button works.

---

### T-037: Match settings passing between states

**Files:** Update all state files, add `MatchConfig` resource.

**Done when:**
- `MatchConfig` resource: `player_hero: HeroId`, `ai_hero: HeroId`, `ai_difficulty: Difficulty`, `surface: CourtSurface`.
- HeroSelect populates `MatchConfig` and inserts it before transitioning to Playing.
- Playing state reads `MatchConfig` to set up heroes and AI.
- PostMatch state reads match results to display score.
- All data flows correctly through the state machine.

**Test:** Full flow: Menu → HeroSelect (pick Mika, Hard) → Play match → PostMatch → Play Again → different hero → verify hero changed.

---

## PHASE 5: Replay System (Tasks 38–42)

### T-038: Replay recorder

**Files:** `crates/ace-client/src/replay/recorder.rs`, `crates/ace-client/src/replay/types.rs`

**Done when:**
- `ReplayRecorder` resource initialized at match start.
- Every physics tick, records a `ReplayFrame`: tick number, both player inputs, ball snapshot, player snapshots.
- Game events (point scored, fault, game won) recorded with tick number.
- Recording stops when match ends.
- `ReplayRecorder` holds the complete `ReplayV0` struct at match end.

**Test:** Play a match → `ReplayRecorder` contains frames. `frames.len() > 0`. Events include point results.

---

### T-039: Replay serialization and hashing

**Files:** `crates/ace-client/src/replay/exporter.rs`

**Done when:**
- `ReplayV0` serialized to bincode bytes.
- SHA-256 hash computed over the bytes.
- Bytes written to `replays/{match_id}.replay`.
- File size logged.
- Hash returned as hex string.

**Test:** After match, call exporter → file appears in `replays/` → file is valid bincode (can be deserialized back) → hash is 64 hex characters.

---

### T-040: Wire "Export Replay" button

**Files:** Update `post_match.rs`

**Done when:**
- "Export Replay" button on post-match screen calls `exporter::export_replay()`.
- After export: button text changes to "Exported ✓".
- Replay hash displayed below button as: "SHA-256: abcd1234...".
- File path displayed: "Saved to: replays/{match_id}.replay".

**Test:** Play match → click Export Replay → file saved → hash displayed → hash matches actual file hash (verify with `shasum -a 256`).

---

### T-041: Canonical result JSON generation

**Files:** `crates/ace-shared/src/protocol.rs`

**Done when:**
- `CanonicalResult` struct containing: match_id, game_id, players (with hero info), score (sets/games/tiebreaks), stats (both players), surface, match_type, duration, ended_at, game_version.
- `CanonicalResult::to_canonical_json() -> String`: serializes with sorted keys, no extra whitespace.
- `CanonicalResult::hash() -> [u8; 32]`: SHA-256 of the canonical JSON bytes.
- Deterministic: same data always produces same hash.

**Test:** Create two identical `CanonicalResult` structs → both produce identical JSON strings → both produce identical hashes. Modify one field → hashes differ.

---

### T-042: Generate result + replay hashes at match end

**Files:** Update `post_match.rs`, integrate `protocol.rs`

**Done when:**
- At match end, both `result_hash` and `replay_hash` are computed.
- Post-match screen shows both hashes.
- Both hashes stored in a `MatchReceipt` resource for blockchain submission.

**Test:** Complete match → both hashes displayed → hashes are valid SHA-256 (64 hex chars each).

---

## PHASE 6: Blockchain Integration (Tasks 43–55)

### T-043: Write AceChainReceipts.sol contract

**Files:** `contracts/src/AceChainReceipts.sol`

**Done when:** Contract matches SPEC-CHAIN-v0.1-BASE.md Section 3.1 exactly. Compiles with `forge build`.

**Test:** `cd contracts && forge build` → no errors.

---

### T-044: Write Foundry tests — core recording

**Files:** `contracts/test/AceChainReceipts.t.sol`

**Done when:** Tests implemented for:
- `test_recordMatch_success`
- `test_recordMatch_emitsEvent`
- `test_recordMatch_storesEnvelopeHash`
- `test_recordMatch_incrementsMatchCount`
- `test_recordMatch_revertsDuplicate`

All pass.

**Test:** `cd contracts && forge test -vv` → 5/5 pass.

---

### T-045: Write Foundry tests — signature verification

**Files:** Update `contracts/test/AceChainReceipts.t.sol`

**Done when:** Tests implemented for:
- `test_recordMatch_revertsUnauthorizedSigner`
- `test_recordMatch_revertsInvalidSignature`

Uses `vm.sign()` to create valid and invalid signatures.

**Test:** `forge test -vv --match-test "Signer|Signature"` → 2/2 pass.

---

### T-046: Write Foundry tests — batching

**Files:** Update `contracts/test/AceChainReceipts.t.sol`

**Done when:** Tests for:
- `test_recordMatches_batchOf5`
- `test_recordMatches_revertsEmptyBatch`
- `test_recordMatches_revertsBatchTooLarge`

**Test:** `forge test -vv --match-test "batch"` → 3/3 pass.

---

### T-047: Write Foundry tests — admin and verification

**Files:** Update `contracts/test/AceChainReceipts.t.sol`

**Done when:** Tests for:
- `test_setSigner_addsNewSigner`
- `test_setSigner_revokesSigner`
- `test_setSigner_onlyOwner`
- `test_pause_stopsRecording`
- `test_unpause_resumesRecording`
- `test_verifyEnvelope_validMatch`
- `test_verifyEnvelope_tamperedData`
- `test_isMatchRecorded`

**Test:** `forge test -vv` → all tests pass (should be ~16+ total now).

---

### T-048: Gas benchmark tests

**Files:** Update `contracts/test/AceChainReceipts.t.sol`

**Done when:**
- `test_gas_singleRecord` measures gas for one `recordMatch()`.
- `test_gas_batchOf25` measures gas for `recordMatches()` with 25 envelopes.
- `test_gas_batchOf100` measures gas for 100 envelopes.
- Gas values printed to console.

**Test:** `forge test -vv --match-test "gas"` → gas values printed, all reasonable (<10M for batch of 100).

---

### T-049: Deploy script

**Files:** `contracts/script/Deploy.s.sol`

**Done when:** Matches SPEC-CHAIN Section 8.2. Deploys contract with owner + initial signer.

**Test:** `forge script script/Deploy.s.sol --rpc-url base_sepolia --broadcast` → contract deployed. Address printed.

---

### T-050: Deploy to Base Sepolia

**Files:** Update `.env` with deployed address

**Done when:**
- Contract deployed to Base Sepolia.
- Contract address saved in `.env` as `RECEIPTS_CONTRACT_ADDRESS`.
- Contract verified on Basescan Sepolia.

**Test:** Visit `https://sepolia.basescan.org/address/{address}` → contract source visible.

---

### T-051: Rust EIP-712 signing module

**Files:** `crates/ace-client/src/blockchain/types.rs`, `crates/ace-client/src/blockchain/signer.rs`

**Done when:**
- `MatchEnvelope` Rust struct matches Solidity struct exactly (using alloy sol! macro).
- `sign_envelope()` function: takes signer private key + envelope → returns ECDSA signature bytes.
- EIP-712 domain constructed with correct name, version, chain ID, contract address.
- Signature format: 65 bytes (r + s + v), compatible with OpenZeppelin ECDSA.recover.

**Test:** Unit test: sign an envelope in Rust → recover signer address → matches expected address.

---

### T-052: Rust transaction submission module

**Files:** `crates/ace-client/src/blockchain/submitter.rs`

**Done when:**
- `submit_match_receipt()` function: takes envelope + signature → submits `recordMatch()` transaction to Base Sepolia.
- Uses alloy provider + signer (same key signs EIP-712 and sends tx, or use separate gas payer).
- Returns: transaction hash on success, error on failure.
- Handles: gas estimation, nonce management, basic retry (1 retry on timeout).

**Test:** Unit test with a mock (don't require live testnet in CI). Integration test: submit a test envelope to Base Sepolia → tx hash returned → event visible on Basescan.

---

### T-053: Wire "Publish to Chain" button

**Files:** Update `crates/ace-client/src/states/post_match.rs`, `crates/ace-client/src/blockchain/mod.rs`

**Done when:**
- "Publish to Chain" button constructs `MatchEnvelope` from match data (result_hash, replay_hash, match_id, etc.).
- Signs envelope with EIP-712.
- Submits transaction.
- During submission: button shows "Publishing..." (disabled).
- On success: button shows "Published ✓", tx hash displayed below.
- Clickable link: `https://sepolia.basescan.org/tx/{txHash}`.
- On error: button shows "Failed — Retry", error message displayed.

**Test:** Play match → click Publish → transaction submitted → tx hash displayed → link works on Basescan → event data matches match results.

---

### T-054: Duplicate submission protection (client-side)

**Files:** Update `post_match.rs`, `submitter.rs`

**Done when:**
- After successful publication, "Publish to Chain" button disabled permanently for that match.
- If user plays same match twice (impossible, but defensive): match_id is UUID, guaranteed unique.
- Client stores published match IDs in memory (not persistent — acceptable for prototype).

**Test:** Publish once → button disabled. Cannot publish same match twice.

---

### T-055: End-to-end integration test

**Files:** `tests/e2e_chain_test.rs` (or bash script)

**Done when:** A single test/script that:
1. Creates a `CanonicalResult` and `ReplayV0` with dummy data.
2. Computes `result_hash` and `replay_hash`.
3. Constructs `MatchEnvelope`.
4. Signs with EIP-712.
5. Submits to Base Sepolia contract.
6. Reads back the event from the transaction receipt.
7. Verifies: event data matches envelope, `isMatchRecorded()` returns true, `verifyEnvelope()` returns true.

**Test:** `cargo test --test e2e_chain_test` → all assertions pass (requires Base Sepolia RPC + funded signer).

---

## PHASE 7: Polish & Bug Fixes (Tasks 56–62)

### T-056: Ball trail visual effect

**Files:** `crates/ace-client/src/systems/vfx.rs`

**Done when:** Ball leaves a short fading trail (10–15 past positions, fading opacity). Trail color indicates spin: white = flat, yellow = topspin, blue = slice.

**Test:** Hit balls → trail visible and correct color per modifier.

---

### T-057: Court bounce mark

**Files:** Update `vfx.rs`

**Done when:** When ball bounces, a small circle mark appears on the court at the bounce position. Mark fades over 3 seconds. Green = in, red = out.

**Test:** Ball bounces → marks appear → in/out colors correct → marks fade.

---

### T-058: Placeholder audio

**Files:** `assets/audio/hit.ogg`, `assets/audio/bounce.ogg`, update Bevy audio setup

**Done when:**
- Ball hit → "hit" sound plays.
- Ball bounce → "bounce" sound plays.
- Sounds are short, distinct, and don't overlap badly.
- Volume reasonable.

**Test:** Play a rally → hear hit and bounce sounds at correct moments.

---

### T-059: Player animations (minimal)

**Files:** Update `movement.rs`, add basic animation states

**Done when:**
- Player model rotates to face movement direction.
- Simple "wind up" and "follow through" rotation on shot execution (rotate upper body toward target).
- No skeletal animation needed — just entity rotation/scaling tricks.

**Test:** Player visually faces direction of movement and shot target. Not standing still while moving.

---

### T-060: Performance profiling

**Files:** Add `bevy_diagnostic` plugin

**Done when:**
- FPS counter displayed (toggle with F3).
- Frame time logged.
- Physics step time measured.
- No frame drops below 60fps on mid-range hardware during normal play.
- If drops found: identify bottleneck (physics? rendering? UI?) and optimize.

**Test:** `cargo run -p ace-client --release` → F3 shows >60 FPS consistently during play.

---

### T-061: Error handling and edge cases

**Files:** Audit all systems

**Done when:**
- Ball that clips through net at extreme angles is caught (backup collision check).
- Ball that lands exactly on a line is "in" (inclusive boundary).
- Score state machine handles all edge cases (tested in T-006).
- Blockchain submission handles: no internet, wrong chain ID, insufficient gas, contract paused.
- App doesn't panic on any tested input sequence.

**Test:** Targeted edge case testing. No panics in 30 minutes of play.

---

### T-062: README and build instructions

**Files:** `README.md`

**Done when:**
- Project description (1 paragraph).
- Prerequisites: Rust, Foundry, Base Sepolia ETH.
- Build instructions: `cargo build --workspace`.
- Run instructions: `cargo run -p ace-client`.
- Contract deployment: link to Deploy.s.sol + forge command.
- `.env` setup guide.
- Link to SPEC files.

**Test:** Fresh clone → follow README → game runs → can play a match → can publish receipt.

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 0 | T-001 – T-006 | Project scaffold, types, scoring |
| 1 | T-007 – T-012 | Court rendering, ball physics |
| 2 | T-013 – T-022 | Mouse aiming, shots, serve |
| 3 | T-023 – T-032 | AI, match loop, HUD |
| 4 | T-033 – T-037 | Menus, hero select |
| 5 | T-038 – T-042 | Replay recording, export |
| 6 | T-043 – T-055 | Blockchain contract, signing, submission |
| 7 | T-056 – T-062 | Polish, VFX, audio, README |

**Total: 62 tasks**

---

*End of Task Breakdown — ACE CHAIN Prototype v0.1*
