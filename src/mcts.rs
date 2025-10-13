use noisy_float::prelude::*;
use rand::prelude::*;

use crate::SupportCard;
use crate::career::{Action, Stat, State, take_action};
use crate::ratings::rating;

const CAREER_LENGTH: u8 = 72;
const MCTS_SIMS: usize = 2000;
const RANDOM_ROLLOUT_MAX_TURNS: Option<u8> = None;

#[derive(Clone, Copy)] // I don't really want copy except to initialize an array
struct MctsStats {
   num_sims: usize,
   total_score: f64,
}

pub fn intelligently_run_career<R: Rng>(state: &mut State, deck: &[SupportCard], rng: &mut R) -> Vec<Action> {
   let mut actions = Vec::new();
   while state.turn < CAREER_LENGTH {
      let possible_actions = [
         Action::Rest,
         Action::Train(Stat::Speed),
         Action::Train(Stat::Stamina),
         Action::Train(Stat::Power),
         Action::Train(Stat::Guts),
         Action::Train(Stat::Wit),
         Action::Recreation,
      ];
      let mut mcts_stats: [MctsStats; 7] = [MctsStats {
         num_sims: 0,
         total_score: 0.0,
      }; 7];
      let mut total_sims = 0;
      while total_sims < MCTS_SIMS {
         let action_to_try = mcts_stats
            .iter()
            .enumerate()
            .max_by_key(|(_, n)| {
               if n.num_sims == 0 {
                  return n64(f64::infinity());
               }
               let avg_rating = n.total_score / n.num_sims as f64;
               let r_prime = avg_rating / 19205.0; // 19205.0 is the max rating by stats only. need to update if we factor in skills.
               n64(r_prime + 2.0.sqrt() * ((total_sims as f64).ln() / n.num_sims as f64).sqrt())
            })
            .map(|(i, _)| i)
            .unwrap();

         let mut s = state.clone();
         take_action(&mut s, possible_actions[action_to_try], deck, rng);

         mcts_stats[action_to_try].num_sims += 1;
         mcts_stats[action_to_try].total_score += random_rollout(s, deck, rng);

         total_sims += 1;
      }
      let best_action_to_take = mcts_stats
         .iter()
         .enumerate()
         .max_by_key(|(_, n)| n.num_sims)
         .map(|(i, _)| possible_actions[i])
         .unwrap();
      actions.push(best_action_to_take);
      take_action(state, best_action_to_take, deck, rng);
   }
   actions
}

fn random_rollout<R: Rng>(mut s: State, deck: &[SupportCard], rng: &mut R) -> f64 {
   let possible_actions = [
      Action::Rest,
      Action::Train(Stat::Speed),
      Action::Train(Stat::Stamina),
      Action::Train(Stat::Power),
      Action::Train(Stat::Guts),
      Action::Train(Stat::Wit),
      Action::Recreation,
   ];
   let start_turn = s.turn;
   loop {
      if s.turn == CAREER_LENGTH {
         break;
      }
      if RANDOM_ROLLOUT_MAX_TURNS
         .map(|max_turns| s.turn - start_turn >= max_turns)
         .unwrap_or(false)
      {
         break;
      }
      take_action(&mut s, *possible_actions.choose(rng).unwrap(), deck, rng);
   }
   rating(&s.stats)
}
