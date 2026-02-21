use serde::{Deserialize, Serialize};

/// Points within a single game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePoints {
    /// Standard game scoring: 0, 15, 30, 40, AD
    /// Points stored as 0..=4 where 4 = advantage.
    Regular { points: [u8; 2] },
    /// Tiebreak scoring: raw point count, first to 7 with 2 clear.
    Tiebreak { points: [u8; 2] },
}

/// Full state of a single-set tennis match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreState {
    pub games: [u8; 2],
    pub points: GamePoints,
    pub server: usize,
    pub tiebreak: bool,
    pub set_complete: bool,
    pub winner: Option<usize>,
    /// Tracks which player served at the start of the tiebreak
    /// for correct serve rotation.
    tiebreak_first_server: usize,
    /// Total tiebreak points played (for serve alternation).
    tiebreak_points_played: u8,
}

impl ScoreState {
    /// Create a fresh score state. Server starts as player 0.
    pub fn new() -> Self {
        Self {
            games: [0, 0],
            points: GamePoints::Regular { points: [0, 0] },
            server: 0,
            tiebreak: false,
            set_complete: false,
            winner: None,
            tiebreak_first_server: 0,
            tiebreak_points_played: 0,
        }
    }

    /// Record a point won by the given player (0 or 1).
    pub fn point_won(&mut self, player: usize) {
        if self.set_complete {
            return;
        }

        let points_snapshot = self.points.clone();
        match points_snapshot {
            GamePoints::Regular { points } => {
                self.handle_regular_point(points, player);
            }
            GamePoints::Tiebreak { points } => {
                self.handle_tiebreak_point(points, player);
            }
        }
    }

    fn handle_regular_point(&mut self, mut points: [u8; 2], player: usize) {
        let opponent = 1 - player;

        if points[player] < 3 {
            // Normal progression: 0->1(15), 1->2(30), 2->3(40)
            points[player] += 1;
            self.points = GamePoints::Regular { points };
        } else if points[player] == 3 && points[opponent] < 3 {
            // Player at 40, opponent below 40 → game won
            self.game_won(player);
        } else if points[player] == 3 && points[opponent] == 3 {
            // 40-40 (deuce) → advantage to player
            points[player] = 4;
            self.points = GamePoints::Regular { points };
        } else if points[player] == 4 {
            // Player has advantage → game won
            self.game_won(player);
        } else if points[player] == 3 && points[opponent] == 4 {
            // Opponent has advantage, player wins point → back to deuce
            points[opponent] = 3;
            self.points = GamePoints::Regular { points };
        }
    }

    fn handle_tiebreak_point(&mut self, mut points: [u8; 2], player: usize) {
        points[player] += 1;
        self.tiebreak_points_played += 1;

        // Check if tiebreak is won: first to 7, must lead by 2
        if points[player] >= 7 && points[player] - points[1 - player] >= 2 {
            // Tiebreak won → game awarded, set complete
            self.points = GamePoints::Tiebreak { points };
            self.games[player] += 1;
            self.set_complete = true;
            self.winner = Some(player);
        } else {
            self.points = GamePoints::Tiebreak { points };
            // Tiebreak serve alternation: first point served by initial server,
            // then every 2 points the server changes.
            if self.tiebreak_points_played % 2 == 1 {
                self.server = if self.server == self.tiebreak_first_server {
                    1 - self.tiebreak_first_server
                } else {
                    self.tiebreak_first_server
                };
            }
        }
    }

    fn game_won(&mut self, player: usize) {
        self.games[player] += 1;

        // Check if set is won
        if self.games[player] >= 6 {
            let diff = self.games[player] as i8 - self.games[1 - player] as i8;
            if diff >= 2 {
                // Set won (6-0, 6-1, 6-2, 6-3, 6-4, 7-5)
                self.set_complete = true;
                self.winner = Some(player);
                self.points = GamePoints::Regular { points: [0, 0] };
                return;
            }
        }

        // Check for tiebreak
        if self.games[0] == 6 && self.games[1] == 6 {
            self.tiebreak = true;
            self.tiebreak_first_server = self.server;
            self.tiebreak_points_played = 0;
            // Server for tiebreak is whoever would serve next (already alternated below
            // conceptually, but we set it before alternation)
            self.server = 1 - self.server;
            self.tiebreak_first_server = self.server;
            self.points = GamePoints::Tiebreak { points: [0, 0] };
            return;
        }

        // Regular game change: alternate server, reset points
        self.server = 1 - self.server;
        self.points = GamePoints::Regular { points: [0, 0] };
    }

    /// Human-readable point display (e.g., "30-15", "Deuce", "Ad-40").
    pub fn display_points(&self) -> String {
        match &self.points {
            GamePoints::Regular { points } => {
                let names = ["0", "15", "30", "40"];

                // Deuce
                if points[0] >= 3 && points[1] >= 3 && points[0] == points[1] {
                    return "Deuce".to_string();
                }

                // Advantage
                if points[0] == 4 {
                    return "Ad-40".to_string();
                }
                if points[1] == 4 {
                    return "40-Ad".to_string();
                }

                format!(
                    "{}-{}",
                    names[points[0] as usize],
                    names[points[1] as usize]
                )
            }
            GamePoints::Tiebreak { points } => {
                format!("{}-{}", points[0], points[1])
            }
        }
    }

    /// Display games score (e.g., "6-4").
    pub fn display_games(&self) -> String {
        format!("{}-{}", self.games[0], self.games[1])
    }
}

impl Default for ScoreState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Normal game progression (0-15-30-40-Game)
    #[test]
    fn test_point_progression() {
        let mut score = ScoreState::new();
        assert_eq!(score.display_points(), "0-0");

        score.point_won(0);
        assert_eq!(score.display_points(), "15-0");

        score.point_won(0);
        assert_eq!(score.display_points(), "30-0");

        score.point_won(0);
        assert_eq!(score.display_points(), "40-0");

        score.point_won(0);
        // Game won, points reset
        assert_eq!(score.games, [1, 0]);
        assert_eq!(score.display_points(), "0-0");
    }

    // 2. Deuce and advantage
    #[test]
    fn test_deuce() {
        let mut score = ScoreState::new();
        // Get to 40-40
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }
        assert_eq!(score.display_points(), "Deuce");
    }

    // 3. Advantage lost → back to deuce
    #[test]
    fn test_advantage_lost() {
        let mut score = ScoreState::new();
        // Get to deuce
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }

        // Advantage to player 0
        score.point_won(0);
        assert_eq!(score.display_points(), "Ad-40");

        // Player 1 wins → back to deuce
        score.point_won(1);
        assert_eq!(score.display_points(), "Deuce");
    }

    // 4. Game won from advantage
    #[test]
    fn test_game_from_advantage() {
        let mut score = ScoreState::new();
        // Get to deuce
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }

        // Advantage to player 0
        score.point_won(0);
        assert_eq!(score.display_points(), "Ad-40");

        // Game won
        score.point_won(0);
        assert_eq!(score.games, [1, 0]);
    }

    // 5. Set won at 6-0
    #[test]
    fn test_set_won_6_0() {
        let mut score = ScoreState::new();
        for _ in 0..6 {
            // Win a game for player 0
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        assert_eq!(score.games, [6, 0]);
        assert!(score.set_complete);
        assert_eq!(score.winner, Some(0));
    }

    // 6. Set won at 6-4
    #[test]
    fn test_set_won_6_4() {
        let mut score = ScoreState::new();
        // Alternate: p0 wins, p1 wins, etc. until 4-4, then p0 wins 2 more
        for i in 0..8 {
            let player = if i % 2 == 0 { 0 } else { 1 };
            for _ in 0..4 {
                score.point_won(player);
            }
        }
        assert_eq!(score.games, [4, 4]);

        // Player 0 wins 2 more games
        for _ in 0..2 {
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        assert_eq!(score.games, [6, 4]);
        assert!(score.set_complete);
        assert_eq!(score.winner, Some(0));
    }

    // 7. Tiebreak triggered at 6-6
    #[test]
    fn test_tiebreak_triggered() {
        let mut score = ScoreState::new();
        // Get to 6-6
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(0);
            }
            for _ in 0..4 {
                score.point_won(1);
            }
        }
        assert_eq!(score.games, [6, 6]);
        assert!(score.tiebreak);
        assert!(matches!(score.points, GamePoints::Tiebreak { .. }));
    }

    // 8. Tiebreak won at 7-5
    #[test]
    fn test_tiebreak_won_7_5() {
        let mut score = ScoreState::new();
        // Get to 6-6
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(0);
            }
            for _ in 0..4 {
                score.point_won(1);
            }
        }

        // Tiebreak: player 0 gets 7, player 1 gets 5
        // Interleave to get to 5-5, then player 0 wins 2
        for _ in 0..5 {
            score.point_won(0);
            score.point_won(1);
        }
        // 5-5 in tiebreak
        score.point_won(0); // 6-5
        score.point_won(0); // 7-5

        assert!(score.set_complete);
        assert_eq!(score.winner, Some(0));
        assert_eq!(score.games, [7, 6]);
    }

    // 9. Tiebreak extended (6-6 → 8-6)
    #[test]
    fn test_tiebreak_extended() {
        let mut score = ScoreState::new();
        // Get to 6-6
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(0);
            }
            for _ in 0..4 {
                score.point_won(1);
            }
        }

        // Tiebreak: get to 6-6
        for _ in 0..6 {
            score.point_won(0);
            score.point_won(1);
        }
        assert!(!score.set_complete);

        // Player 0 wins 2 to get 8-6
        score.point_won(0); // 7-6
        assert!(!score.set_complete);
        score.point_won(0); // 8-6
        assert!(score.set_complete);
        assert_eq!(score.winner, Some(0));
    }

    // 10. Server alternation each game
    #[test]
    fn test_server_alternation() {
        let mut score = ScoreState::new();
        assert_eq!(score.server, 0);

        // Player 0 wins first game
        for _ in 0..4 {
            score.point_won(0);
        }
        assert_eq!(score.server, 1);

        // Player 1 wins second game
        for _ in 0..4 {
            score.point_won(1);
        }
        assert_eq!(score.server, 0);
    }

    // 11. Tiebreak server alternation (every 2 points)
    // Pattern: A serves 1 point, then B serves 2, A serves 2, B serves 2, ...
    #[test]
    fn test_tiebreak_server_alternation() {
        let mut score = ScoreState::new();
        // Get to 6-6
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(0);
            }
            for _ in 0..4 {
                score.point_won(1);
            }
        }

        let first_server = score.server;

        // Point 1: first server serves, then server changes
        score.point_won(0);
        let after_1 = score.server;
        assert_ne!(after_1, first_server, "Server should change after 1st point");

        // Point 2: second server serves, no change yet
        score.point_won(1);
        let after_2 = score.server;
        assert_eq!(after_2, after_1, "Server should stay same after 2nd point");

        // Point 3: server changes back
        score.point_won(0);
        let after_3 = score.server;
        assert_ne!(after_3, after_2, "Server should change after 3rd point");
        assert_eq!(after_3, first_server, "Server should be back to first server");

        // Point 4: no change
        score.point_won(1);
        let after_4 = score.server;
        assert_eq!(after_4, after_3, "Server should stay same after 4th point");

        // Point 5: server changes
        score.point_won(0);
        let after_5 = score.server;
        assert_ne!(after_5, after_4, "Server should change after 5th point");
    }

    // 12. Set complete flag set correctly
    #[test]
    fn test_set_complete_flag() {
        let mut score = ScoreState::new();

        // At 5-0 the set should not be complete
        for _ in 0..5 {
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        assert!(!score.set_complete);
        assert_eq!(score.games, [5, 0]);

        // At 6-0 the set should be complete
        for _ in 0..4 {
            score.point_won(0);
        }
        assert!(score.set_complete);
    }

    // 13. Winner set correctly
    #[test]
    fn test_winner_set() {
        let mut score = ScoreState::new();

        // Player 1 wins 6-0
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(1);
            }
        }
        assert_eq!(score.winner, Some(1));
    }

    // 14. Multiple games in sequence
    #[test]
    fn test_multiple_games_sequence() {
        let mut score = ScoreState::new();

        // Play 3 games: p0, p1, p0
        for _ in 0..4 {
            score.point_won(0);
        }
        assert_eq!(score.games, [1, 0]);

        for _ in 0..4 {
            score.point_won(1);
        }
        assert_eq!(score.games, [1, 1]);

        for _ in 0..4 {
            score.point_won(0);
        }
        assert_eq!(score.games, [2, 1]);
    }

    // 15. Full set simulation (randomized, check invariants)
    #[test]
    fn test_full_set_simulation() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let mut score = ScoreState::new();
            let mut total_points = 0;

            while !score.set_complete {
                let player = rng.gen_range(0..2);
                score.point_won(player);
                total_points += 1;

                // Safety: a set should never exceed ~200 points
                assert!(total_points < 500, "Set took too many points");
            }

            // Invariants
            assert!(score.winner.is_some());
            let winner = score.winner.unwrap();
            let loser = 1 - winner;

            // Winner must have at least 6 games
            assert!(score.games[winner] >= 6);

            // If tiebreak, winner has 7, loser has 6
            if score.tiebreak {
                assert_eq!(score.games[winner], 7);
                assert_eq!(score.games[loser], 6);
            } else {
                // Non-tiebreak: winner leads by at least 2
                assert!(score.games[winner] >= score.games[loser] + 2);
            }
        }
    }

    // 16. No set at 5-4
    #[test]
    fn test_no_set_at_5_4() {
        let mut score = ScoreState::new();
        // p0 wins 5, p1 wins 4 (alternating with p0 getting one extra)
        for i in 0..9 {
            let player = if i < 5 { 0 } else { 1 };
            for _ in 0..4 {
                score.point_won(player);
            }
        }
        // Note: server alternation means this isn't exactly 5-4 due to ordering
        // Let's do it more carefully:
        let mut score = ScoreState::new();
        for _ in 0..5 {
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        assert_eq!(score.games[0], 5);
        for _ in 0..4 {
            for _ in 0..4 {
                score.point_won(1);
            }
        }
        assert_eq!(score.games, [5, 4]);
        assert!(!score.set_complete);
    }

    // 17. Set won at 7-5
    #[test]
    fn test_set_won_7_5() {
        let mut score = ScoreState::new();
        // Get to 5-5
        for _ in 0..5 {
            for _ in 0..4 {
                score.point_won(0);
            }
            for _ in 0..4 {
                score.point_won(1);
            }
        }
        assert_eq!(score.games, [5, 5]);

        // Player 0 wins to 6-5
        for _ in 0..4 {
            score.point_won(0);
        }
        assert_eq!(score.games, [6, 5]);
        assert!(!score.set_complete);

        // Player 1 ties to 6-6? No — let's have player 0 win to 7-5
        for _ in 0..4 {
            score.point_won(0);
        }
        assert_eq!(score.games, [7, 5]);
        assert!(score.set_complete);
        assert_eq!(score.winner, Some(0));
    }

    // 18. Display points - regular
    #[test]
    fn test_display_points_regular() {
        let mut score = ScoreState::new();
        score.point_won(0);
        score.point_won(0);
        score.point_won(1);
        assert_eq!(score.display_points(), "30-15");
    }

    // 19. Display points - deuce
    #[test]
    fn test_display_points_deuce() {
        let mut score = ScoreState::new();
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }
        assert_eq!(score.display_points(), "Deuce");
    }

    // 20. Display points - advantage
    #[test]
    fn test_display_points_advantage() {
        let mut score = ScoreState::new();
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }
        score.point_won(0);
        assert_eq!(score.display_points(), "Ad-40");

        // Reset and test advantage for player 1
        let mut score = ScoreState::new();
        for _ in 0..3 {
            score.point_won(0);
        }
        for _ in 0..3 {
            score.point_won(1);
        }
        score.point_won(1);
        assert_eq!(score.display_points(), "40-Ad");
    }

    // 21. Display games
    #[test]
    fn test_display_games() {
        let mut score = ScoreState::new();
        for _ in 0..3 {
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        for _ in 0..2 {
            for _ in 0..4 {
                score.point_won(1);
            }
        }
        assert_eq!(score.display_games(), "3-2");
    }

    // 22. Initial state
    #[test]
    fn test_initial_state() {
        let score = ScoreState::new();
        assert_eq!(score.games, [0, 0]);
        assert_eq!(score.points, GamePoints::Regular { points: [0, 0] });
        assert_eq!(score.server, 0);
        assert!(!score.tiebreak);
        assert!(!score.set_complete);
        assert_eq!(score.winner, None);
    }

    // 23. Points don't advance after set is complete
    #[test]
    fn test_no_points_after_set_complete() {
        let mut score = ScoreState::new();
        for _ in 0..6 {
            for _ in 0..4 {
                score.point_won(0);
            }
        }
        assert!(score.set_complete);
        let games_before = score.games;

        score.point_won(0);
        assert_eq!(score.games, games_before);
    }
}
