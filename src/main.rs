#![feature(variant_count)]

mod career;
mod mcts;
mod ratings;
mod support_cards;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::{self};
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use noisy_float::prelude::*;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use crate::career::{Action, deal_supports, new_career_state};
use crate::mcts::intelligently_run_career;
use crate::ratings::rating;
use crate::support_cards::SupportCard;

#[derive(Default)]
struct Result {
   deck: Vec<SupportCard>,
   rating: f64,
   actions: Vec<Action>,
   stats: [u16; 5],
}

fn main() {
   let mut support_card_pool: Vec<SupportCard> = serde_json::from_slice(&std::fs::read("cards.json").unwrap()).unwrap();
   support_card_pool.retain(|x| x.rarity > 1 && x.limit_break == 4 && x.r#type <= 4);
   //support_card_pool.retain(|x| x.r#type == 0 || x.r#type == 4);

   let (sender, receiver) = mpsc::sync_channel::<Result>(16);

   for _ in 0..15 {
      let sender = sender.clone();
      let mut support_card_pool = support_card_pool.clone();
      std::thread::spawn(move || {
         let mut rng = XorShiftRng::from_os_rng();
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

            sender
               .send(Result {
                  deck: deck.to_vec(),
                  rating: rtg,
                  actions,
                  stats: state.stats,
               })
               .unwrap();
         }
      });
   }

   let mut best_result = Result::default();

   struct RunningMean {
      mean: f64,
      n: u64,
   }

   let mut tier_list: IndexMap<u32, RunningMean> = IndexMap::new();
   let support_cards: HashMap<u32, SupportCard> = support_card_pool.iter().map(|x| (x.id, x.clone())).collect();

   let mut last_time_tierlist_written = Instant::now();

   loop {
      let maybe_msg = receiver.recv_timeout(Duration::from_secs(30));
      match maybe_msg {
         Ok(msg) => {
            for card in msg.deck.iter() {
               let entry = tier_list.entry(card.id).or_insert(RunningMean { mean: 0.0, n: 0 });
               entry.n += 1;
               entry.mean += (msg.rating - entry.mean) / entry.n as f64;
            }
            if msg.rating > best_result.rating {
               best_result = msg;
               println!("New best rating: {:.2}", best_result.rating);
               let mut f = File::create("best.txt").unwrap();
               for action in &best_result.actions {
                  writeln!(f, "{:?}", action).unwrap();
               }
               for card in &best_result.deck {
                  writeln!(f, "{}", card).unwrap();
               }
               writeln!(f, "{:?}", best_result.stats).unwrap();
               writeln!(f, "{}", best_result.rating).unwrap();
            }
            if last_time_tierlist_written.elapsed() >= Duration::from_secs(30) {
               let mut f = File::create("tierlist.html").unwrap();
               f.write_all(HTML_HEADER.as_bytes()).unwrap();
               tier_list.sort_unstable_by_key(|_, v| std::cmp::Reverse(n64(v.mean)));
               let tiers = [
                  ("SS", 6150),
                  ("S", 6050),
                  ("A", 5950),
                  ("B", 5850),
                  ("C", 5750),
                  ("D", 0),
               ];
               let mut current_tier_idx = 0;
               let mut new_tier = true;
               for (k, v) in tier_list.iter() {
                  while v.mean < tiers[current_tier_idx].1 as f64 {
                     current_tier_idx += 1;
                     new_tier = true;
                  }
                  if new_tier {
                     if current_tier_idx != 0 {
                        writeln!(f, "</div></div>").unwrap();
                     }
                     writeln!(f, "<div class=\"tier\" id=\"{}\">", tiers[current_tier_idx].0).unwrap();
                     writeln!(f, "<h2>{}</h2>", tiers[current_tier_idx].0).unwrap();
                     writeln!(f, "<div class=\"icons\">").unwrap();
                     new_tier = false;
                  }
                  writeln!(
                     f,
                     "<img src=\"./cardImages/support_card_s_{}.png\" alt=\"{} - {:.2}\">",
                     support_cards[k].id, support_cards[k], v.mean
                  )
                  .unwrap();
               }
               writeln!(f, "</div></div>").unwrap();
               f.write_all(HTML_FOOTER.as_bytes()).unwrap();
               last_time_tierlist_written = Instant::now();
            }
         }
         Err(_) => break,
      }
   }
}

const HTML_HEADER: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>tier list</title>
  <style>
    body {
      background: #111;
      color: #eee;
      font-family: monospace;
      margin: 2rem;
    }
    h1 { margin-bottom: 1rem; }
    .tier {
      margin-bottom: 1.5rem;
    }
    .tier h2 {
      font-size: 1rem;
      margin: 0.5rem 0;
    }
    .icons {
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
    }
    img {
      width: 64px;
      height: 64px;
      object-fit: cover;
      border-radius: 4px;
    }
  </style>
</head>
"##;

const HTML_FOOTER: &str = r##"
</body>
</html>
"##;
