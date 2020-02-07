mod game;
mod ai;

use ai::{ Game, ActionTree };
use game::{ Player, TicTacToe };

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// A newtype wrapper for TicTacToe to do handle `wasm_bindgen`'s inability to make bindings for generic impls.
#[wasm_bindgen]
pub struct Board(TicTacToe);

#[wasm_bindgen]
impl Board {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Board(TicTacToe::new())
    }

    pub fn draw(&self, ctx: &CanvasRenderingContext2d, size: f64) -> Result<(), JsValue> {
        self.0.draw(ctx, size)
    }

    pub fn action_for_click(&mut self, x: f64, y: f64, board_size: f64) -> Option<u8> {
        self.0.action_for_click(x, y, board_size)
    }

    pub fn do_action_mut(&mut self, action: u8) {
        self.0.do_action_mut(&action);
    }

    pub fn current_player(&self) -> String {
        match self.0.current_player() {
            Player::X => "X".to_owned(),
            Player::O => "O".to_owned()
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.0.game_over()
    }

    pub fn reset(&mut self) {
        self.0 = TicTacToe::new();
    }
}

/// Holds statistics about an action to be sent to Javascript for UTTTMonteCarloAI::get_best_action
#[wasm_bindgen]
pub struct ActionStats {
    pub action: u8,
    pub sims: u32,
    pub wins: u32
}

/// A newtype wrapper for `ActionTree<TicTacToe>` that allows JavaScript to control an ActionTree specifically for
/// Ultimate TicTacToe. This is necessary because `#[wasm_bindgen]` doesn't work on generic impls.
#[wasm_bindgen]
pub struct UTTTMonteCarloAI(ActionTree<TicTacToe>);

#[wasm_bindgen]
impl UTTTMonteCarloAI {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        UTTTMonteCarloAI(ActionTree::new(TicTacToe::new()))
    }

    pub fn do_search_step(&mut self, num_sims: u32) {
        self.0.do_search_step(num_sims);
    }

    pub fn get_best_action(&self) -> Option<ActionStats> {
        self.0.get_best_action()
            .map(|(action, node_id)| ActionStats {
                action: *action,
                sims: self.0.get_node_total_points(node_id),
                wins: self.0.get_node_earned_points(node_id)
            })
    }

    pub fn do_action(&mut self, action: u8) {
        self.0.do_action(&action);
    }

    pub fn current_player(&self) -> String {
        match self.0.current_player() {
            Player::X => "X".to_owned(),
            Player::O => "O".to_owned()
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.0.is_game_over()
    }

    pub fn reset(&mut self) {
        self.0 = ActionTree::new(TicTacToe::new());
    }
}