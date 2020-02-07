use std::collections::{ HashMap, HashSet };
use std::hash::Hash;
use slab::Slab;
use rand::prelude::*;
use rand::rngs::SmallRng;

const EXPLORATION_FACTOR: f64 = 1.4142135623730950488016887242097; // sqrt(2)

/// A problem which agents can work on. An object implementing this trait should contain the system's state.
pub trait Game: Clone {
    type Action: Hash + Eq + Clone;
    type Player: Hash + Eq + Clone;

    /// Returns a list of actions that can be taken on the game in its current state
    fn available_actions(&self) -> &[Self::Action];
    /// Performs an action immutably, returning a copy of this object that has had the action applied to it.
    /// Assumes that the given action is valid (i.e. it was returned from Game::available_actions).
    fn do_action(&self, action: &Self::Action) -> Box<Self>;
    /// Performs an action mutably, applying the action to this object.
    /// Assumes that the given action is valid (i.e. it was returned from Game::available_actions)
    fn do_action_mut(&mut self, action: &Self::Action);
    /// Gets a list of all players in the game
    fn get_players(&self) -> &[Self::Player];
    /// Returns the player that is currently allowed to make a move
    fn current_player(&self) -> Self::Player;
    /// If a player has won the game then this returns the winner, otherwise it returns None.
    fn winner(&self) -> Option<Self::Player>;
    /// Returns whether the game has ended
    fn game_over(&self) -> bool { self.available_actions().is_empty() }
}

/// Plays `num_sims` games starting from `base_state` with each player performing a random action each turn.
/// Returns the number of times each player wins one of the simulated games.
fn simulate<G: Game, R: Rng>(rng: &mut R, base_state: &G, num_sims: u32) -> (u32, HashMap<G::Player, u32>) {
    let mut points = base_state.get_players().iter()
        .map(|player| (player.clone(), 0))
        .collect::<HashMap<G::Player, u32>>();

    for _ in 0..num_sims {
        let mut state = base_state.clone();

        // Make random moves
        loop {
            if let Some(action) = state.available_actions().choose(rng).cloned() {
                state.do_action_mut(&action);
            } else {
                // no more possible moves, the game is over
                break;
            }
        }

        // Update the win count, unless the game tied and there isn't a winner
        if let Some(winner) = state.winner() {
            // If there was a winner, give them 10 points
            *points.get_mut(&winner).unwrap() += 10;
        } else {
            // Otherwise it was a draw. Give each player one point
            for x in points.values_mut() {
                *x += 1;
            }
        }
    }
    (10 * num_sims, points)
}

pub struct ActionTree<G: Game> {
    rng: SmallRng,
    nodes: Slab<ActionTreeNode<G>>,
    root: usize
}

struct ActionTreeNode<G: Game> {
    id: usize,
    state: G,

    total_points: u32,
    earned_points: u32,
    score: f64,

    parent: Option<usize>,
    children: HashMap<G::Action, usize>
}

impl<G: Game> ActionTree<G> {
    pub fn new(state: G) -> Self {
        let mut tree = ActionTree {
            rng: SmallRng::seed_from_u64(0),
            nodes: Slab::new(),
            root: 0 // temporarily
        };
        tree.set_root(state);
        tree
    }

    fn set_root(&mut self, state: G) {
        let entry = self.nodes.vacant_entry();
        let key = entry.key();
        entry.insert(ActionTreeNode {
            id: key,
            state: state,

            total_points: 0,
            earned_points: 0,
            score: std::f64::INFINITY,

            parent: None,
            children: HashMap::new()
        });
        self.root = key;
    }

    /// Selects the node that should be simulated next by following the path with the highest scores
    fn select(&self) -> usize {
        let mut current_node_id = self.root;

        loop {
            let current_node = self.nodes.get(current_node_id).unwrap();

            // if this node has no children, then we can't continue
            if current_node.children.is_empty() {
                return current_node_id;
            }

            // find the child with maximal score
            let best_child = current_node.children.values()
                .map(|id| self.nodes.get(*id).unwrap())
                .max_by(|node_a, node_b| node_a.score.partial_cmp(&node_b.score).unwrap())
                .unwrap();
            
            // continue with the best child
            current_node_id = best_child.id;
        }
    }

    /// Creates a child node of a given node for each action that can be performed on that node's state.
    /// Returns the ID of one of the children, or the id of this node if no children were created, for use when choosing
    /// a node to simulate.
    fn expand(&mut self, node_id: usize) -> usize {
        // Get information from the node that is being expanded
        // We have to do this in its own block so we can release the borrow on the parent node before inserting the children
        let parent_state = {
            let node = self.nodes.get(node_id).unwrap();
            node.state.clone()
        };

        // Create a child node for each available action on the parent's state and collect the children's IDs into a HashMap
        let children = parent_state.available_actions().iter().map(|action| {
            let entry = self.nodes.vacant_entry();
            let key = entry.key();
            entry.insert(ActionTreeNode {
                id: key,
                state: *parent_state.do_action(&action),

                total_points: 0,
                earned_points: 0,
                score: std::f64::INFINITY,

                parent: Some(node_id),
                children: HashMap::new()
            });
            (action.clone(), key)
        }).collect();

        let node = self.nodes.get_mut(node_id).unwrap();
        node.children = children;
        node.children.values().nth(0).copied().unwrap_or(node_id)
    }

    /// Backpropagates the results of a simulation, updating the winrate statistics for all nodes in the path from the
    /// simulated node to the root.
    fn backpropagate(&mut self, node_id: usize, total_points: u32, earned_points: HashMap<G::Player, u32>) {
        let mut node = self.nodes.get_mut(node_id).unwrap();
        let mut path = Vec::new();

        // Build a path from the leaf node back to the root
        loop {
            // Add this node to the path
            path.push(node.id);

            // Continue to the parent
            if let Some(parent_id) = node.parent {
                node = self.nodes.get_mut(parent_id).unwrap();
            } else {
                // this was the root, we're done backpropagating
                break;
            }
        }

        // Follow the path from the root back to the leaf, updaing each nodes scores as we go
        // This is done seperately from the last step so that we can hold onto the parent's simulation count, which is
        // used in the score function, and the parent's current player, which is who the winrate should be calculated for
        let mut parent_player = node.state.current_player();
        let mut parent_total_points = node.total_points as f64;
        for id in path.iter().rev() {
            node = self.nodes.get_mut(*id).unwrap();

            // Update simulation statistics
            node.total_points += total_points;
            node.earned_points += earned_points.get(&parent_player).unwrap_or(&0);

            let total_points = node.total_points as f64;
            let points = node.earned_points as f64;
            // UCT score (see https://en.wikipedia.org/wiki/Monte_Carlo_tree_search#Exploration_and_exploitation)
            node.score = (points / total_points) + EXPLORATION_FACTOR * (parent_total_points.ln() / total_points).sqrt();

            parent_player = node.state.current_player();
            parent_total_points = total_points;
        }
    }

    /// Performs a single step of the Monte Carlo tree search algorithm.
    /// (See https://en.wikipedia.org/wiki/Monte_Carlo_tree_search#Principle_of_operation)
    pub fn do_search_step(&mut self, num_sims: u32) {
        // Select a node to simulate
        let mut node_to_sim = self.select();
        
        // If this node has already been simulated, then we should expand it and simulate one of the children instead
        let should_expand = {
            if let Some(node) = self.nodes.get(node_to_sim) {
                node.total_points > 0
            } else {
                false
            }
        };

        // Expand the node if necessary
        if should_expand {
            node_to_sim = self.expand(node_to_sim);
        }

        if let Some(node) = self.nodes.get(node_to_sim) {
            // Do the simulation
            let (total_points, wins) = simulate(&mut self.rng, &node.state, num_sims);

            // Backpropagate the simulation results
            self.backpropagate(node_to_sim, total_points, wins);
        }

    }

    /// Gets the action that provides the best estimated winrate for the current player.
    pub fn get_best_action(&self) -> Option<(&G::Action, usize)> {
        let root = self.nodes.get(self.root).unwrap();

        let mut best_winrate = 0.0;
        let mut best_action = None;
        for (action, child_id) in root.children.iter() {
            let child = self.nodes.get(*child_id).unwrap();
            let winrate = child.earned_points as f64 / child.total_points as f64;
            if winrate > best_winrate {
                best_winrate = winrate;
                best_action = Some((action, *child_id));
            }
        }

        best_action
    }

    /// Removes any nodes that can no longer be reached from the root node
    fn collect_garbage(&mut self) {
        // Mark all of the nodes that can be reached from the root
        let mut marked_nodes = HashSet::new();
        let mut openset = vec![self.root];
        while !openset.is_empty() {
            // Take a node from the openset & mark it
            let id = openset.pop().unwrap();
            marked_nodes.insert(id);
            // Add all children of that node to the openset
            let node = self.nodes.get(id).unwrap();
            openset.extend(node.children.values());
        }

        // Find all unmarked nodes
        let to_remove = self.nodes.iter()
            .map(|(key, _)| key)
            .filter(|key| !marked_nodes.contains(key))
            .collect::<Vec<usize>>();

        // Remove all unmarked nodes
        for id in to_remove {
            self.nodes.remove(id);
        }
    }

    pub fn do_action(&mut self, action: &G::Action) {
        // Find the ID of the new root among the current root's children
        let root = self.nodes.get(self.root).unwrap();
        if let Some(new_root_id) = root.children.get(action) {
            // Set the tree's root to the new root
            self.root = *new_root_id;
            // Clear the new root's parent
            let new_root = self.nodes.get_mut(self.root).unwrap();
            new_root.parent = None;
        } else {
            // A node for this child doesn't exist yet, so we should make one
            let next_state = root.state.do_action(action);
            self.set_root(*next_state);
        }
        // This will make some nodes unreachable, so remove them
        self.collect_garbage();
    }

    pub fn get_node_earned_points(&self, node: usize) -> u32 {
        self.nodes.get(node).unwrap().earned_points
    }

    pub fn get_node_total_points(&self, node: usize) -> u32 {
        self.nodes.get(node).unwrap().total_points
    }

    pub fn is_game_over(&self) -> bool {
        self.nodes.get(self.root).unwrap().state.game_over()
    }

    pub fn current_player(&self) -> G::Player {
        self.nodes.get(self.root).unwrap().state.current_player()
    }
}