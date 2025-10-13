#![feature(variant_count)]

mod career;
mod mcts;
mod ratings;
mod support_cards;

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::time::Duration;

use atomic_float::AtomicF64;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use crate::career::{Action, deal_supports, new_career_state};
use crate::mcts::intelligently_run_career;
use crate::ratings::rating;
use crate::support_cards::SupportCard;

fn try_report_score(
   rating: f64,
   actions: &[Action],
   deck: &[SupportCard],
   stats: &[u16],
   known_rating: f64,
   atomic: &AtomicF64,
   sender: &SyncSender<Result>,
) -> f64 {
   let mut prev_val = known_rating;
   let mut updated = false;
   loop {
      match atomic.compare_exchange_weak(prev_val, rating, Ordering::Relaxed, Ordering::Relaxed) {
         Ok(_) => {
            updated = true;
            break;
         }
         Err(v) => {
            if v >= rating {
               break;
            }
            prev_val = v;
         }
      }
   }
   if updated {
      sender
         .send(Result {
            rating,
            actions: actions.to_vec(),
            deck: deck.to_vec(),
            stats: stats.try_into().unwrap(),
         })
         .unwrap();
   }
   prev_val
}

#[derive(Default)]
struct Result {
   deck: Vec<SupportCard>,
   rating: f64,
   actions: Vec<Action>,
   stats: [u16; 5],
}

static BEST_RATING: AtomicF64 = AtomicF64::new(0.0);

fn main() {
   let mut support_card_pool: Vec<SupportCard> = serde_json::from_slice(&std::fs::read("cards.json").unwrap()).unwrap();
   support_card_pool.retain(|x| x.rarity > 1 && x.limit_break == 4 && x.r#type <= 4);

   println!("{}", support_card_pool.len());

   let (sender, receiver) = mpsc::sync_channel::<Result>(16);

   for _ in 0..8 {
      let sender = sender.clone();
      let mut support_card_pool = support_card_pool.clone();
      std::thread::spawn(move || {
         let mut rng = XorShiftRng::from_os_rng();
         let mut known_best_rating = 0.0;
         loop {
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

            if rtg > known_best_rating {
               known_best_rating = try_report_score(
                  rtg,
                  &actions,
                  deck,
                  &state.stats,
                  known_best_rating,
                  &BEST_RATING,
                  &sender,
               )
            }
         }
      });
   }

   let mut best_result = Result::default();
   let mut dirty = false;

   loop {
      let maybe_msg = receiver.recv_timeout(Duration::from_secs(30));
      match maybe_msg {
         Ok(msg) => {
            best_result = msg;
            println!("New best rating: {:.2}", best_result.rating);
            dirty = true;
         }
         Err(RecvTimeoutError::Timeout) => {
            if dirty {
               for action in &best_result.actions {
                  println!("{:?}", action);
               }
               for card in &best_result.deck {
                  println!("{}", card);
               }
               println!("{:?}", best_result.stats);
               println!("{}", best_result.rating);
            }
         }
         Err(RecvTimeoutError::Disconnected) => {
            if dirty {
               for action in best_result.actions {
                  println!("{:?}", action);
               }
               for card in best_result.deck {
                  println!("{}", card);
               }
               println!("{:?}", best_result.stats);
               println!("{}", best_result.rating);
            }
            break;
         }
      }
   }
}
