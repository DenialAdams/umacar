use num_traits::PrimInt;
use rand::prelude::*;

use crate::SupportCard;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Stat {
   Speed = 0,
   Stamina = 1,
   Power = 2,
   Guts = 3,
   Wit = 4,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
   Rest,
   Train(Stat),
   Recreation,
}

#[derive(Clone, Copy)]
pub enum Mood {
   Awful,
   Bad,
   Normal,
   Good,
   Great,
}

impl Mood {
   fn next(&self) -> Mood {
      match self {
         Mood::Awful => Mood::Bad,
         Mood::Bad => Mood::Normal,
         Mood::Normal => Mood::Good,
         Mood::Good | Mood::Great => Mood::Great,
      }
   }

   fn prior(&self) -> Mood {
      match self {
         Mood::Awful | Mood::Bad => Mood::Awful,
         Mood::Normal => Mood::Bad,
         Mood::Good => Mood::Normal,
         Mood::Great => Mood::Good,
      }
   }

   fn as_modifier(&self) -> f64 {
      match self {
         Mood::Great => 0.2,
         Mood::Good => 0.1,
         Mood::Normal => 0.0,
         Mood::Bad => -0.1,
         Mood::Awful => -0.2,
      }
   }
}

pub type Stats = [u16; std::mem::variant_count::<Stat>()];

#[derive(Clone)]
pub struct State {
   pub energy: u8,
   pub mood: Mood,
   pub stats: Stats,
   pub turn: u8,
   pub times_trained: [u8; std::mem::variant_count::<Stat>()],
   pub friendship: [u8; 6],
   pub support_locations: [Stat; std::mem::variant_count::<Stat>()],
}

fn add_with_cap<T: PrimInt>(v1: T, v2: T, cap: T) -> T {
   (v1 + v2).min(cap)
}

// First stat matters, rest is oredered arbitrarily
const PRIO: [[Stat; 5]; 5] = [
   [Stat::Speed, Stat::Stamina, Stat::Power, Stat::Guts, Stat::Wit],
   [Stat::Stamina, Stat::Speed, Stat::Power, Stat::Guts, Stat::Wit],
   [Stat::Power, Stat::Speed, Stat::Stamina, Stat::Guts, Stat::Wit],
   [Stat::Guts, Stat::Speed, Stat::Stamina, Stat::Power, Stat::Wit],
   [Stat::Wit, Stat::Speed, Stat::Stamina, Stat::Power, Stat::Guts],
];

pub fn deal_supports<R: Rng>(s: &mut State, deck: &[SupportCard], rng: &mut R) {
   for (card, location) in deck.iter().zip(s.support_locations.iter_mut()) {
      let stat_prio = PRIO[card.r#type as usize];
      // TODO: kitasan unique effect
      let denom = 550 + card.specialty_rate;
      let val = rng.random_range(0..denom);
      if val < card.specialty_rate + 110 {
         *location = stat_prio[0];
      } else if val < card.specialty_rate + 220 {
         *location = stat_prio[1];
      } else if val < card.specialty_rate + 330 {
         *location = stat_prio[2];
      } else if val < card.specialty_rate + 440 {
         *location = stat_prio[3];
      } else {
         *location = stat_prio[4];
      }
   }
}

pub fn take_action<R: Rng>(s: &mut State, action: Action, deck: &[SupportCard], rng: &mut R) {
   match action {
      Action::Rest => {
         // https://gamewith.net/uma-musume/69549
         // numbers are clearly not true values, but should be good enough
         let v = rng.random_range(0u16..3833);
         if v < 975 {
            s.energy = add_with_cap(s.energy, 70, 100);
         } else if v < 3201 {
            s.energy = add_with_cap(s.energy, 50, 100);
         } else {
            s.energy = add_with_cap(s.energy, 30, 100);
            if v >= 3693 {
               s.mood = s.mood.prior(); // TODO confirm this actually happens
               // TODO: Night Owl
            }
         }
      }
      Action::Train(training_stat) => {
         let failure_chance = if s.energy < 50 { 0.8 } else { 0.0 }; // TODO
         if rng.random_bool(1.0 - failure_chance) {
            let stat_values_for_training: [f64; 5] = match training_stat {
               Stat::Speed => [10.0, 0.0, 5.0, 0.0, 0.0],
               Stat::Stamina => [0.0, 10.0, 0.0, 5.0, 0.0],
               Stat::Power => [0.0, 5.0, 10.0, 0.0, 0.0],
               Stat::Guts => [5.0, 0.0, 5.0, 10.0, 0.0],
               Stat::Wit => [2.0, 0.0, 0.0, 0.0, 9.0],
            }; // TODO - these are totally bogus
            let training_level = (s.times_trained[training_stat as usize] / 4).min(4);
            let num_ppl_here = s
               .support_locations
               .iter()
               .filter(|where_at| **where_at == training_stat)
               .count();
            let sum_training_effectiveness: f64 = s
               .support_locations
               .iter()
               .enumerate()
               .filter(|(_, where_at)| **where_at == training_stat)
               .map(|(i, _)| deck[i].tb - 1.0)
               .sum::<f64>();
            let sum_friendship_bonus: f64 = s
               .support_locations
               .iter()
               .enumerate()
               .filter(|(i, where_at)| {
                  **where_at == training_stat && deck[*i].r#type == training_stat as u8 && s.friendship[*i] >= 80
               })
               .map(|(i, _)| deck[i].fs_bonus - 1.0)
               .sum::<f64>();
            let sum_mood_bonus: f64 = s
               .support_locations
               .iter()
               .enumerate()
               .filter(|(_, where_at)| **where_at == training_stat)
               .map(|(i, _)| deck[i].mb - 1.0)
               .sum::<f64>();
            let friendship_multiplier = 1.0 + sum_friendship_bonus;
            let mood_multiplier = 1.0 + (s.mood.as_modifier() * (1.0 + sum_mood_bonus));
            let effectiveness_mulitiplier = 1.0 + sum_training_effectiveness;
            let ppl_here_multiplier = 1.0 + (num_ppl_here as f64 * 0.05);
            let growth_rate = 1.0; // TODO
            for (training_val, stat) in stat_values_for_training
               .iter()
               .zip([Stat::Speed, Stat::Stamina, Stat::Power, Stat::Guts, Stat::Wit])
               .filter(|(tv, _)| **tv != 0.0)
            {
               let base_training_value = training_val + training_level as f64; // TODO - wrong - how to find it?
               let bonus_training_value = s
                  .support_locations
                  .iter()
                  .enumerate()
                  .filter(|(_, where_at)| **where_at == training_stat)
                  .map(|(i, _)| &deck[i].stat_bonus[stat as usize])
                  .sum::<u8>() as f64;
               let final_training_value = (base_training_value + bonus_training_value)
                  * friendship_multiplier
                  * mood_multiplier
                  * effectiveness_mulitiplier
                  * ppl_here_multiplier
                  * growth_rate;
               s.stats[stat as usize] = add_with_cap(s.stats[stat as usize], final_training_value.round() as u16, 1200);
            }
            s.times_trained[training_stat as usize] = s.times_trained[training_stat as usize].saturating_add(1);

            // Adjust friendship
            for card_here in s
               .support_locations
               .iter()
               .enumerate()
               .filter(|(_, where_at)| **where_at == training_stat)
               .map(|(i, _)| i)
            {
               s.friendship[card_here] = add_with_cap(s.friendship[card_here], 7, 100);
            }

            // Adjust energy
            let energy_adjustment_for_training_level = match training_level {
               0 => 0,
               1 => 1,
               2 => 2,
               3 => 4,
               4 => 6,
               _ => unreachable!(),
            };
            match training_stat {
               Stat::Speed => {
                  s.energy = s.energy.saturating_sub(21 + energy_adjustment_for_training_level);
               }
               Stat::Stamina => {
                  s.energy = s.energy.saturating_sub(19 + energy_adjustment_for_training_level);
               }
               Stat::Power => {
                  s.energy = s.energy.saturating_sub(20 + energy_adjustment_for_training_level);
               }
               Stat::Guts => {
                  s.energy = s.energy.saturating_sub(22 + energy_adjustment_for_training_level);
               }
               Stat::Wit => {
                  let sum_wis_recovery: u8 = s
                     .support_locations
                     .iter()
                     .enumerate()
                     .filter(|(_, stat)| **stat == Stat::Wit)
                     .map(|(i, _)| &deck[i].wisdom_recovery)
                     .sum();
                  s.energy = add_with_cap(s.energy, 5 + sum_wis_recovery, 100);
               }
            }
         } else {
            // TODO
            s.mood = s.mood.prior();
            s.stats[training_stat as usize] = s.stats[training_stat as usize].saturating_sub(10);
         }
      }
      Action::Recreation => {
         s.mood = s.mood.next();
         // TODO: energy recovery
      }
   }
   s.turn += 1;
   deal_supports(s, deck, rng);
}

pub fn new_career_state(deck: &[SupportCard]) -> State {
   let mut state = State {
      energy: 100,
      mood: Mood::Normal,
      stats: [100; 5],
      turn: 0,
      times_trained: [0; 5],
      friendship: [0; 6],
      support_locations: [Stat::Speed, Stat::Speed, Stat::Speed, Stat::Speed, Stat::Speed],
   };
   for (i, card) in deck.iter().enumerate() {
      for (j, initial_boost) in card.starting_stats.iter().enumerate() {
         state.stats[j] = add_with_cap(state.stats[j], *initial_boost, 1200);
      }

      state.friendship[i] = add_with_cap(state.friendship[i], card.sb, 100);
   }
   state
}
