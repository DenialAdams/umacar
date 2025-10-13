#![feature(variant_count)]

mod career;
mod mcts;
mod ratings;
mod support_cards;

use std::collections::HashSet;

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use crate::career::{Action, deal_supports, new_career_state};
use crate::mcts::intelligently_run_career;
use crate::ratings::rating;
use crate::support_cards::SupportCard;

fn main() {
   let mut support_card_pool: Vec<SupportCard> = serde_json::from_slice(&std::fs::read("cards.json").unwrap()).unwrap();
   support_card_pool.retain(|x| x.rarity > 1 && x.limit_break == 4 && x.r#type <= 4);

   let mut rng = XorShiftRng::from_os_rng();
   //let mut rng = XorShiftRng::from_seed([123; 16]);

   let mut best_deck: Vec<SupportCard> = Vec::new();
   let mut best_rating: f64 = 0.0;
   let mut best_actions: Vec<Action> = Vec::new();
   let mut best_stats: [u16; 5] = [0; 5];

   // Try some decks
   for _ in 0..10 {
      'create_deck: loop {
         support_card_pool.shuffle(&mut rng);
         let mut chars_in_deck: HashSet<&str> = HashSet::new();
         for card in support_card_pool[0..6].iter() {
            if !chars_in_deck.insert(&card.char_name) {
               continue 'create_deck;
            }
         }
         break;
      }
      let deck = &support_card_pool[0..6];
      let mut state = new_career_state(deck);
      deal_supports(&mut state, deck, &mut rng);
      let actions = intelligently_run_career(&mut state, deck, &mut rng);
      let rtg = rating(&state.stats);
      if rtg > best_rating {
         best_rating = rtg;
         best_deck = Vec::from(deck);
         best_actions = actions;
         best_stats = state.stats;
      }
   }
   for action in best_actions {
      println!("{:?}", action);
   }
   for card in best_deck {
      println!("{}", card);
   }
   println!("{:?}", best_stats);
   println!("{}", best_rating);
}
