import * as wasm from "uttt-ai";

wasm.set_panic_hook();

var steps_per_round = 1;
var sim_time = 0;
var total_sims = 0;

var options = {
    target_round_time: 100,
    simulations_per_step: 1000,
    simulation_enabled: false,
    thinking_time: 10000,
    playing_for: { X: false, O: false }
}

var ai = new wasm.UTTTMonteCarloAI();

onmessage = function(e) {
    var request = e.data;

    if (request.type == "get_action") {
        action_requested = true;
    } else if (request.type == "do_action") {
        ai.do_action(request.action);
        sim_time = 0;
    } else if (request.type == "set_options") {
        for (var opt in request.options) {
            if (request.options[opt] == 'toggle') {
                options[opt] = !options[opt]
            } else {
                options[opt] = request.options[opt]
            }
        }
    } else if (request.type = "new_game") {
        ai.reset();
    }
}

function do_simulations() {
    if (ai.is_game_over()) {
        options.simulation_enabled = false;
    }

    if (!options.simulation_enabled) {
        setTimeout(do_simulations, options.target_round_time);
        return;
    }

    var sim_start = Date.now();

    for (var i = 0; i < steps_per_round; i++) {
        ai.do_search_step(options.simulations_per_step);
    }

    var sim_count = steps_per_round * options.simulations_per_step;
    var round_time = Date.now() - sim_start;

    // Update steps_per_round to try to make the next round of simulations take target_round_time milliseconds
    steps_per_round = Math.floor(options.target_round_time / (round_time / steps_per_round));

    total_sims += sim_count;
    sim_time += round_time;
    var sim_rate = sim_count / (round_time / 1000)

    var stats = ai.get_best_action();
    if (stats) {
        postMessage({
            type: "stats",
            best_action: stats.action,
            sim_time: sim_time,
            sims: stats.sims,
            wins: stats.wins,
            round_sim_count: sim_count,
            total_sims: total_sims,
            sim_rate: sim_rate,
        });

        if (sim_time >= options.thinking_time && options.playing_for[ai.current_player()]) {
            postMessage({
                type: "do_action",
                action: stats.action
            });
            ai.do_action(stats.action);
            sim_time = 0;
        }
    }

    // Use setTimeout to yield so that messages can be processed
    setTimeout(do_simulations, 0);
}
do_simulations();