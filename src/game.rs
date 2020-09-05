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
    board_x: u128,
    board_o: u128,
    // Keeps track of which players have won which sub-boards
    winners_x: u16,
    winners_o: u16,
    // The index of the sub-board that the current player is able to play in. If they can play in any board, then this is set to None.
    active_board: Option<u8>,
    // Cached set of available actions
    available_actions: Vec<u8>,
    
    current_player: Player,
    game_over: bool
}

const WIN_MASKS: [u16; 8] = [
    0b111000000,
    0b000111000,
    0b000000111,
    0b100100100,
    0b010010010,
    0b001001001,
    0b100010001,
    0b001010100
];

// Checks whether a player has won a given board and if so returns that player.
fn check_for_winner(board: u16) -> bool {
    return WIN_MASKS.iter().any(|&mask| mask & board == mask)
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
            board_x: 0,
            board_o: 0,
            winners_x: 0,
            winners_o: 0,
            active_board: None,
            available_actions: Vec::new(),
            current_player: Player::X,
            game_over: false
        };
        board.update_available_actions();
        board
    }

    pub fn update_available_actions(&mut self) {
        //self.available_actions.clear();

        if self.game_over {
            // no possible actions if someone has already won
            return;
        }

        let available_spaces = !(self.board_x | self.board_o);
        let available_subboards = !(self.winners_x | self.winners_o);

        self.available_actions = if let Some(board_i) = self.active_board {
            let board_start = board_i * 9;
            (board_start..board_start + 9)
                .filter(|&i| available_spaces & (1 << i) != 0)
                .collect()
        } else {
            (0..81)
                .filter(|&i| available_subboards & (1 << (i / 9)) != 0)
                .filter(|&i| available_spaces & (1 << i) != 0)
                .collect()
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
        for board_i in 0..9 {
            let board_x = board_size * (board_i % 3) as f64;
            let board_y = board_size * (board_i / 3) as f64;
            ctx.save();
            ctx.translate(board_x, board_y)?;

            ctx.set_stroke_style(&BLACK.into());
            draw_grid(ctx, board_size);

            for cell_i in 0..9 {
                let cell_x = cell_size * (cell_i % 3) as f64;
                let cell_y = cell_size * (cell_i / 3) as f64;
                // Translate to the center of this cell
                ctx.save();
                ctx.translate(cell_x + cell_size / 2.0, cell_y + cell_size / 2.0)?;

                let cell_mask = 1u128 << (cell_i + board_i * 9);
                if self.board_x & cell_mask != 0 {
                    draw_x(ctx, cell_size);
                }
                if self.board_o & cell_mask != 0 {
                    draw_o(ctx, cell_size)?;
                }

                ctx.restore();
            }

            ctx.restore();
        }

        // Draw symbols for winners over boards they've won.
        ctx.set_line_width(6.0);
        for board_i in 0..9 {
            let board_x = board_size * (board_i % 3) as f64;
            let board_y = board_size * (board_i / 3) as f64;
            // Translate to the center of the board
            ctx.save();
            ctx.translate(board_x + board_size / 2.0, board_y + board_size / 2.0)?;
            
            let cell_mask = 1u16 << board_i;
            if self.winners_x & cell_mask != 0 {
                draw_x(ctx, board_size);
            }
            if self.winners_o & cell_mask != 0 {
                draw_o(ctx, board_size)?;
            }

            ctx.restore();
        }

        Ok(())
    }

    pub fn action_for_click(&mut self, x: f64, y: f64, board_size: f64) -> Option<u8> {
        let cell_x = x * 9.0 / board_size;
        let cell_y = y * 9.0 / board_size;

        if cell_x < 0.0 || cell_y < 0.0 || cell_x >= 9.0 || cell_y >= 9.0 {
            return None;
        }

        let board_i = ((cell_x / 3.0).floor() + 3.0 * (cell_y / 3.0).floor()) as u8;
        let cell_i = ((cell_x % 3.0).floor() + 3.0 * (cell_y % 3.0).floor()) as u8;

        let action = cell_i + board_i * 9;

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
        let board_i = *action / 9;
        let cell_i = *action % 9;

        // Put the symbol on the board
        let player_board = match self.current_player {
            Player::X => {
                self.board_x |= 1u128 << action;
                self.board_x
            },
            Player::O => {
                self.board_o |= 1u128 << action;
                self.board_o
            }
        };

        // Check if this causes the current player to win this board
        // Isolate the specific subboard the action modified
        let subboard = player_board >> (9 * board_i);
        if check_for_winner((subboard & 0x1FF) as u16) {
            let winner_board = match self.current_player {
                Player::X => {
                    self.winners_x |= 1u16 << board_i;
                    self.winners_x
                },
                Player::O => {
                    self.winners_o |= 1u16 << board_i;
                    self.winners_o
                }
            };

            // Check if this causes the current player to win the game
            if check_for_winner(winner_board) {
                self.game_over = true;
                self.current_player = self.current_player;
                self.update_available_actions();
                return;
            }
        }

        // Set the active board
        let board_mask = 1 << board_i;
        if (self.winners_x | self.winners_o) & board_mask != 0 {
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
        if self.game_over {
            Some(self.current_player)
        } else {
            None
        }
    }

    fn game_over(&self) -> bool {
        self.game_over
    }
}