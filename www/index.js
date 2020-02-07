import * as wasm from "uttt-ai";

wasm.set_panic_hook();

class UIManager {
    constructor(game_manager) {
        this.game_manager = game_manager;
        this.last_stats = null;

        this.game_settings_panel = document.getElementById("new-game-settings");
        this.game_stats_panel = document.getElementById("game-stats");

        this.player_select_x = document.getElementById("player-select-x");
        this.player_select_o = document.getElementById("player-select-o");
        this.start_game_button = document.getElementById("start-game")

        this.current_player = document.getElementById("current-player");

        this.thinking_time = document.getElementById("thinking-time");
        this.best_action = document.getElementById("best-action");
        this.winrate = document.getElementById("winrate");
        this.sim_count = document.getElementById("sim-count");
        this.sim_rate = document.getElementById("sim-rate");

        this.pause_button = document.getElementById("pause")

        this.canvas_container = document.getElementById("canvas-container");
        this.canvas = document.getElementById("main-canvas");
        this.canvas_ctx = this.canvas.getContext("2d");

        this.board_x = 0;
        this.board_y = 0;
        this.board_size = 0;



        const self = this;

        window.addEventListener("resize", function() {
            self.onWindowResize()
        });
        this.onWindowResize();
        
        this.start_game_button.addEventListener("click", function() {
            self.game_manager.new_game({
                players: {
                    X: self.player_select_x.value,
                    O: self.player_select_o.value
                }
            });
            self.game_settings_panel.style.display = "none";
            self.game_stats_panel.style.display = null;
        })

        this.pause_button.addEventListener("click", function() {
            self.game_manager.set_worker_options({
                simulation_enabled: 'toggle'
            });
        });

        this.canvas.addEventListener("click", function(e) {
            self.game_manager.handle_click(e.clientX - self.board_x, e.clientY - self.board_y, self.board_size);
        });
        
    }

    onWindowResize() {
        this.canvas.width = this.canvas_container.offsetWidth;
        this.canvas.height = window.innerHeight;
        this.render_board();
    }

    update_game_info() {
        var player = this.game_manager.board.current_player();
        this.current_player.textContent = player;
        if (player == "X") {
            this.current_player.classList.remove("player-o")
            this.current_player.classList.add("player-x");
        } else {
            this.current_player.classList.remove("player-x")
            this.current_player.classList.add("player-o");
        }

        // Show the new game panel if the game ended
        if (this.game_manager.board.is_game_over()) {
            this.game_settings_panel.style.display = null;
            this.game_stats_panel.style.display = "none";
        }
    }

    update_stats(stats) {
        this.thinking_time.textContent = Math.floor(stats.sim_time / 100) / 10

        // Only show the best action when the AI is playing (don't let human players cheat!)
        if (this.game_manager.current_player_type == "ai") {
            var action_a = stats.best_action >> 4;
            var action_b = stats.best_action & 0xF;
            this.best_action.textContent = "(" + action_a + ", " + action_b + ")";
        } else {
            this.best_action.textContent = "<hidden>";
        }
        
        var winrate = stats.wins / stats.sims;
        this.winrate.textContent = Math.floor(winrate * 100) + "%";
        this.winrate.style.color = "rgb(" + Math.floor((1 - winrate) * 255) + "," + Math.floor(winrate*255) + ",0)";
        
        this.sim_count.textContent = Math.floor(stats.total_sims / 1000) + "k";
        
        this.sim_rate.textContent = Math.floor(stats.sim_rate);
    
        this.last_stats = stats;
        this.render_board();
    }

    render_board() {
        this.board_size = Math.min(this.canvas.width, this.canvas.height) * 0.9;
        this.board_x = this.canvas.width / 2 - this.board_size / 2;
        this.board_y = this.canvas.height / 2 - this.board_size / 2;
    
        this.canvas_ctx.resetTransform();
        this.canvas_ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        this.canvas_ctx.translate(this.board_x, this.board_y);
        this.game_manager.board.draw(this.canvas_ctx, this.board_size);
    
        // Only show the best action when the AI is playing (don't let human players cheat!)
        if (this.last_stats && this.game_manager.current_player_type == "ai") {
            var action_a = this.last_stats.best_action >> 4;
            var action_b = this.last_stats.best_action & 0xF;
    
            var cell_col = 3*Math.floor(action_a % 3) + Math.floor(action_b % 3);
            var cell_row = 3*Math.floor(action_a / 3) + Math.floor(action_b / 3);
    
            var cell_x = cell_col * this.board_size / 9 + this.board_size / 18;
            var cell_y = cell_row * this.board_size / 9 + this.board_size / 18;
    
            var box_size = (this.board_size / 9) * 0.8;
    
            this.canvas_ctx.fillStyle = "rgba(0, 255, 0, 0.3)";
            this.canvas_ctx.fillRect(cell_x - box_size / 2, cell_y - box_size / 2, box_size, box_size);
        }
    }
}

class GameManager {
    constructor() {
        this.board = new wasm.Board();
        this.players = { X: "human", O: "human" };
        this.worker = new Worker("./bootstrap-worker.js");
        this.ui = new UIManager(this);

        const self = this;

        this.worker.onmessage = function(e) {
            var msg = e.data;
            if (msg.type == "do_action") {
                self.do_action(msg.action);
            } else if (msg.type == "stats") {
                self.ui.update_stats(msg);
            }
        }
    }

    get current_player_type() {
        return this.players[this.board.current_player()]
    }

    set_worker_options(options) {
        this.worker.postMessage({
            type: "set_options",
            options: options
        })
    }

    do_action(action) {
        this.board.do_action_mut(action);
        this.ui.last_stats = null; // clear last_stats to remove the best_move marker
        this.ui.render_board();
        this.ui.update_game_info();
    }

    handle_click(x, y, board_size) {
        if (this.current_player_type != "human") {
            // Human player can only play for X
            return;
        }
    
        var action = this.board.action_for_click(x, y, board_size);
        if (action != null) {
            this.do_action(action);
            this.worker.postMessage({
                type: "do_action",
                action: action
            });
            this.ui.render_board();
        }
    }

    new_game(settings) {
        this.board.reset();
        this.players = settings.players;
        this.worker.postMessage({
            type: "new_game"
        });
        this.worker.postMessage({
            type: "set_options",
            options: {
                playing_for: {
                    X: this.players.X == "ai",
                    O: this.players.O == "ai"
                },
                simulation_enabled: this.players.X == "ai" || this.players.O == "ai"
            }
        })
    }
}

window.game_manager = new GameManager();