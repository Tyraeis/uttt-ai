use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use crate::ai::Game;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Player {
    X, O
}

#[derive(Clone, Debug)]
pub struct TicTacToe {
    // The current state of the game board
    board: [[Option<Player>; 9]; 9],
    // Keeps track of which players have won which sub-boards
    winners: [Option<Player>; 9],
    // The index of the sub-board that the current player is able to play in. If they can play in any board, then this is set to None.
    active_board: Option<usize>,
    // Cached set of available actions
    available_actions: Vec<u8>,
    
    current_player: Player,
    game_over: bool
}

fn encode_action(action: &(usize, usize)) -> u8 {
    (action.0 as u8) << 4 | (action.1 as u8)
}

fn decode_action(action: u8) -> (usize, usize) {
    ((action >> 4) as usize, (action & 0xF) as usize)
}

// Checks whether any player has three spaces in a line. Used in check_for_winner
fn check_line(a: Option<Player>, b: Option<Player>, c: Option<Player>) -> Option<Player> {
    if a == b && b == c {
        a
    } else {
        None
    }
}

// Checks whether a player has won a given board and if so returns that player.
fn check_for_winner(board: [Option<Player>; 9]) -> Option<Player> {
    for i in 0..3 {
        // Check columns
        if let Some(player) = check_line(board[i], board[i+3], board[i+6]) {
            return Some(player)

        // Check rows
        } else if let Some(player) = check_line(board[3*i], board[3*i + 1], board[3*i + 2]) {
            return Some(player)
        }
    }

    // Check diagonals
    if let Some(player) = check_line(board[0], board[4], board[8]) {
        return Some(player)
    } else if let Some(player) = check_line(board[2], board[4], board[6]) {
        return Some(player)
    }

    None
}

const BLACK: &str = "#000";
const RED: &str = "#f00";
const BLUE: &str = "#00f";
const LIGHT_RED: &str = "#fcc";
const LIGHT_BLUE: &str = "#ccf";

fn line(ctx: &CanvasRenderingContext2d, x1: f64, y1: f64, x2: f64, y2: f64) {
    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.line_to(x2, y2);
    ctx.stroke();
}

fn draw_grid(ctx: &CanvasRenderingContext2d, grid_size: f64) {
    let cell_size = grid_size / 3.0;
    // Vertical lines
    line(ctx, cell_size, 0.0, cell_size, grid_size);
    line(ctx, 2.0 * cell_size, 0.0, 2.0 * cell_size, grid_size);
    // Horizontal lines
    line(ctx, 0.0, cell_size, grid_size, cell_size);
    line(ctx, 0.0, 2.0 * cell_size, grid_size, 2.0 * cell_size);
}

fn draw_x(ctx: &CanvasRenderingContext2d, size: f64) {
    let offset = size / 2.0 * 0.8;
    ctx.set_stroke_style(&RED.into());
    line(ctx, -offset, -offset, offset, offset);
    line(ctx, offset, -offset, -offset, offset);
}

fn draw_o(ctx: &CanvasRenderingContext2d, size: f64) -> Result<(), JsValue> {
    ctx.set_stroke_style(&BLUE.into());
    ctx.begin_path();
    ctx.arc(0.0, 0.0, size / 2.0 * 0.8, 0.0, 2.0 * std::f64::consts::PI)?;
    ctx.stroke();
    Ok(())
}

impl TicTacToe {
    pub fn new() -> Self {
        let mut board = TicTacToe {
            board: [[None; 9]; 9],
            winners: [None; 9],
            active_board: None,
            available_actions: Vec::new(),
            current_player: Player::X,
            game_over: false
        };
        board.update_available_actions();
        board
    }

    pub fn update_available_actions(&mut self) {
        self.available_actions.clear();

        if self.game_over {
            // no possible actions if someone has already won
            return;
        }

        if let Some(board_i) = self.active_board {
            for (cell_i, cell) in self.board[board_i].iter().enumerate() {
                if cell.is_none() {
                    self.available_actions.push(encode_action(&(board_i, cell_i)));
                }
            }
        } else {
            for (board_i, board) in self.board.iter().enumerate() {
                if self.winners[board_i] == None {
                    for (cell_i, cell) in board.iter().enumerate() {
                        if cell.is_none() {
                            self.available_actions.push(encode_action(&(board_i, cell_i)));
                        }
                    }
                }
            }
        }
    }

    // Draws the board onto an HTML canvas with the upper-left corner at (0, 0).
    pub fn draw(&self, ctx: &CanvasRenderingContext2d, size: f64) -> Result<(), JsValue> {
        // Highlight the active sub-board.
        if !self.game_over {
            match self.current_player {
                Player::X => ctx.set_fill_style(&LIGHT_RED.into()),
                Player::O => ctx.set_fill_style(&LIGHT_BLUE.into())
            }
            
            if let Some(i) = self.active_board {
                ctx.fill_rect(
                    size / 3.0 * (i % 3) as f64, size / 3.0 * (i / 3) as f64,
                    size / 3.0, size / 3.0
                );
            } else {
                // No restriction; highlight the entire board.
                ctx.fill_rect(
                    0.0, 0.0,
                    size, size
                );
            }
        }

        // Draw large board.
        ctx.set_stroke_style(&BLACK.into());
        ctx.set_line_width(6.0);
        draw_grid(ctx, size);

        // Draw small boards.
        ctx.set_line_width(2.0);
        let board_size = size / 3.0;
        let cell_size = size / 9.0;
        for (board_i, board) in self.board.iter().enumerate() {
            let board_x = board_size * (board_i % 3) as f64;
            let board_y = board_size * (board_i / 3) as f64;
            ctx.save();
            ctx.translate(board_x, board_y)?;

            ctx.set_stroke_style(&BLACK.into());
            draw_grid(ctx, board_size);

            for (cell_i, cell) in board.iter().enumerate() {
                let cell_x = cell_size * (cell_i % 3) as f64;
                let cell_y = cell_size * (cell_i / 3) as f64;
                // Translate to the center of this cell
                ctx.save();
                ctx.translate(cell_x + cell_size / 2.0, cell_y + cell_size / 2.0)?;

                match cell {
                    &Some(Player::X) => {
                        draw_x(ctx, cell_size);
                    }
                    &Some(Player::O) => {
                        draw_o(ctx, cell_size)?;
                    }
                    &None => { /* empty cell */ }
                }

                ctx.restore();
            }

            ctx.restore();
        }

        // Draw symbols for winners over boards they've won.
        ctx.set_line_width(6.0);
        for (board_i, winner) in self.winners.iter().enumerate() {
            if let &Some(player) = winner {
                let board_x = board_size * (board_i % 3) as f64;
                let board_y = board_size * (board_i / 3) as f64;
                // Translate to the center of the board
                ctx.save();
                ctx.translate(board_x + board_size / 2.0, board_y + board_size / 2.0)?;

                match player {
                    Player::X => {
                        draw_x(ctx, board_size);
                    }
                    Player::O => {
                        draw_o(ctx, board_size)?;
                    }
                }

                ctx.restore();
            }
        }

        Ok(())
    }

    pub fn action_for_click(&mut self, x: f64, y: f64, board_size: f64) -> Option<u8> {
        let cell_x = x * 9.0 / board_size;
        let cell_y = y * 9.0 / board_size;

        if cell_x < 0.0 || cell_y < 0.0 || cell_x >= 9.0 || cell_y >= 9.0 {
            return None;
        }

        let action = encode_action(&(
            ((cell_x / 3.0).floor() + 3.0 * (cell_y / 3.0).floor()) as usize,
            ((cell_x % 3.0).floor() + 3.0 * (cell_y % 3.0).floor()) as usize
        ));

        if self.available_actions.contains(&action) {
            Some(action)
        } else {
            None
        }
    }
}

const TIC_TAC_TOE_PLAYERS: [Player; 2] = [Player::X, Player::O];

impl Game for TicTacToe {
    type Action = u8;
    type Player = Player;

    fn available_actions(&self) -> &[Self::Action] {
        &self.available_actions
    }

    fn do_action(&self, action: &Self::Action) -> Box<Self> {
        let mut c = self.clone();
        c.do_action_mut(action);
        Box::new(c)
    }

    fn do_action_mut(&mut self, action: &Self::Action) {
        let (board_i, cell_i) = decode_action(*action);

        // Put the symbol on the board
        self.board[board_i][cell_i] = Some(self.current_player);

        // Check if this causes the current player to win this board
        if let Some(winner_1) = check_for_winner(self.board[board_i]) {
            self.winners[board_i] = Some(winner_1);

            // Check if this causes the current player to win the game
            if let Some(winner_2) = check_for_winner(self.winners) {
                self.game_over = true;
                self.current_player = winner_2;
                self.update_available_actions();
                return;
            }
        }

        // Set the active board
        if self.winners[cell_i].is_none() {
            self.active_board = Some(cell_i);
        } else {
            self.active_board = None;
        }

        // Toggle player
        self.current_player = match self.current_player {
            Player::X => Player::O,
            Player::O => Player::X
        };

        // Update set of available actions
        self.update_available_actions();

        // Check if the game is a draw (no available actions)
        if self.available_actions.is_empty() {
            self.game_over = true;
        }
    }

    fn get_players(&self) -> &[Self::Player] {
        &TIC_TAC_TOE_PLAYERS
    }

    fn current_player(&self) -> Self::Player {
        self.current_player
    }

    fn winner(&self) -> Option<Self::Player> {
        check_for_winner(self.winners)
    }

    fn game_over(&self) -> bool {
        self.game_over
    }
}