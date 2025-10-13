#![feature(variant_count)]

use std::fmt::Display;

use noisy_float::prelude::*;
use num_traits::PrimInt;
use rand::{
    Rng, SeedableRng,
    seq::{IndexedRandom, SliceRandom},
};
use rand_xorshift::XorShiftRng;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SupportCard {
    pub id: u32,
    #[serde(rename = "type")]
    pub r#type: u8,
    pub group: bool,
    pub rarity: u32,
    pub limit_break: u32,
    pub starting_stats: [u32; 5],
    pub type_stats: u32,
    pub stat_bonus: [u32; 6],
    pub race_bonus: u32,
    pub sb: u32,
    pub specialty_rate: u32,
    pub unique_specialty: f64,
    pub offstat_appearance_denominator: u32,
    pub tb: f64,
    pub fs_bonus: f64,
    pub mb: f64,
    pub unique_fs_bonus: f64,
    pub fs_stats: [u32; 6],
    pub fs_training: u32,
    pub fs_motivation: u32,
    pub fs_specialty: u32,
    pub fs_ramp: [u32; 2],
    pub fs_energy: u32,
    pub wisdom_recovery: u8,
    pub effect_size_up: f64,
    pub energy_up: f64,
    pub energy_discount: f64,
    pub fail_rate_down: f64,
    pub hint_rate: f64,
    pub highlander_threshold: u32,
    pub highlander_training: u32,
    pub crowd_bonus: u32,
    pub char_name: String,
    pub fan_bonus: u32,
}

impl Display for SupportCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({})",
            self.char_name,
            match self.rarity {
                1 => "R",
                2 => "SR",
                3 => "SSR",
                _ => unreachable!(),
            },
            match self.r#type {
                0 => "Speed",
                1 => "Stamina",
                2 => "Power",
                3 => "Guts",
                4 => "Wit",
                _ => "unreachable",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
enum Stat {
    Speed = 0,
    Stamina = 1,
    Power = 2,
    Guts = 3,
    Wit = 4,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Rest,
    Train(Stat),
    Recreation,
}

#[derive(Clone, Copy)]
enum Mood {
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

type Stats = [u16; std::mem::variant_count::<Stat>()];

const CAREER_LENGTH: u8 = 72;

#[derive(Clone)]
struct State {
    energy: u8,
    mood: Mood,
    stats: Stats,
    turn: u8,
    times_trained: [u8; std::mem::variant_count::<Stat>()],
    friendship: [u8; 6],
    support_locations: [Stat; std::mem::variant_count::<Stat>()],
}

fn add_with_cap<T: PrimInt>(v1: T, v2: T, cap: T) -> T {
    (v1 + v2).min(cap)
}

// First stat matters, rest is oredered arbitrarily
const PRIO: [[Stat; 5]; 5] = [
    [
        Stat::Speed,
        Stat::Stamina,
        Stat::Power,
        Stat::Guts,
        Stat::Wit,
    ],
    [
        Stat::Stamina,
        Stat::Speed,
        Stat::Power,
        Stat::Guts,
        Stat::Wit,
    ],
    [
        Stat::Power,
        Stat::Speed,
        Stat::Stamina,
        Stat::Guts,
        Stat::Wit,
    ],
    [
        Stat::Guts,
        Stat::Speed,
        Stat::Stamina,
        Stat::Power,
        Stat::Wit,
    ],
    [
        Stat::Wit,
        Stat::Speed,
        Stat::Stamina,
        Stat::Power,
        Stat::Guts,
    ],
];

fn deal_supports<R: Rng>(s: &mut State, deck: &[SupportCard], rng: &mut R) {
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

fn take_action<R: Rng>(s: &mut State, action: Action, deck: &[SupportCard], rng: &mut R) {
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
                    Stat::Wit => [5.0, 0.0, 0.0, 0.0, 10.0],
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
                    .map(|(i, _)| &deck[i].tb)
                    .sum::<f64>()
                    - num_ppl_here as f64;
                let sum_mood_bonus: f64 = s
                    .support_locations
                    .iter()
                    .enumerate()
                    .filter(|(_, where_at)| **where_at == training_stat)
                    .map(|(i, _)| &deck[i].mb)
                    .sum::<f64>()
                    - num_ppl_here as f64;
                let friendship_multiplier = 1.0; // TODO
                let mood_multiplier = 1.0 + (s.mood.as_modifier() * (1.0 + sum_mood_bonus));
                let effectiveness_mulitiplier = 1.0 + sum_training_effectiveness;
                let ppl_here_multiplier = 1.0 + (num_ppl_here as f64 * 0.05);
                let growth_rate = 1.0; // TODO
                for (training_val, stat) in stat_values_for_training
                    .iter()
                    .zip([
                        Stat::Speed,
                        Stat::Stamina,
                        Stat::Power,
                        Stat::Guts,
                        Stat::Wit,
                    ])
                    .filter(|(tv, _)| **tv != 0.0)
                {
                    //println!("{}", sum_mood_bonus);
                    let base_training_value = training_val + training_level as f64; // TODO - wrong - how to find it?
                    // TODO add support card stat bonus
                    let final_training_value = base_training_value
                        * friendship_multiplier
                        * mood_multiplier
                        * effectiveness_mulitiplier
                        * ppl_here_multiplier
                        * growth_rate;
                    s.stats[stat as usize] = add_with_cap(
                        s.stats[stat as usize],
                        final_training_value.round() as u16,
                        1200,
                    );
                }
                s.times_trained[training_stat as usize] =
                    s.times_trained[training_stat as usize].saturating_add(1);
                match training_stat {
                    Stat::Speed => {
                        s.energy = s.energy.saturating_sub(20);
                    }
                    Stat::Stamina => {
                        s.energy = s.energy.saturating_sub(20);
                    }
                    Stat::Power => {
                        s.energy = s.energy.saturating_sub(20);
                    }
                    Stat::Guts => {
                        s.energy = s.energy.saturating_sub(20);
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
                s.stats[training_stat as usize] =
                    s.stats[training_stat as usize].saturating_sub(10);
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

fn rating(s: &Stats) -> f64 {
    // https://pastebin.com/qSfsjwGC from https://docs.google.com/spreadsheets/d/1AAZCVDMiCozDNKte-5JYSm7SWUeVJ0o87i1iDY1bnWM/edit?usp=sharing
    s.iter().copied().map(rate_stat).sum()
}

fn rate_stat(mut t: u16) -> f64 {
    const F: [f64; 25] = [
        0.5, 0.8, 1.0, 1.3, 1.6, 1.8, 2.1, 2.4, 2.6, 2.8, 2.9, 3.0, 3.1, 3.3, 3.4, 3.5, 3.9, 4.1,
        4.2, 4.3, 5.2, 5.5, 6.6, 6.8, 6.9,
    ];
    const M: [f64; 81] = [
        7.888, 8.0, 8.1, 8.3, 8.4, 8.5, 8.6, 8.8, 8.9, 9.0, 9.2, 9.3, 9.4, 9.6, 9.7, 9.8, 10.0,
        10.1, 10.2, 10.3, 10.5, 10.6, 10.7, 10.9, 11.0, 11.1, 11.3, 11.4, 11.5, 11.7, 11.8, 11.9,
        12.1, 12.2, 12.3, 12.4, 12.6, 12.7, 12.8, 13.0, 13.1, 13.2, 13.4, 13.5, 13.6, 13.8, 13.9,
        14.0, 14.1, 14.3, 14.4, 14.5, 14.7, 14.8, 14.9, 15.1, 15.2, 15.3, 15.5, 15.6, 15.7, 15.9,
        16.0, 16.1, 16.2, 16.4, 16.5, 16.6, 16.8, 16.9, 17.0, 17.2, 17.3, 17.4, 17.6, 17.7, 17.8,
        17.9, 18.1, 18.2, 18.3,
    ];

    if t == 1643 {
        return 8587.0;
    }
    if t == 1865 {
        return 11931.0;
    }

    if t <= 1200 {
        t += 1;
        let mut a = 0.0;
        for e in F {
            if t <= 50 {
                a += t as f64 * e;
                break;
            }
            a += 50.0 * e;
            t -= 50;
        }
        a.floor()
    } else if t > 1200 && t <= 1209 {
        ((t - 1200) as f64 * M[0]).ceil() + 3841.0
    } else if t > 2000 {
        unimplemented!()
    } else {
        t = t - 1210 + 1;
        let mut n = 0.0;
        for v in M {
            if t <= 10 {
                n += (t as f64 * v).ceil();
                break;
            }
            n += (10.0 * v).ceil();
            t -= 10;
        }
        n + 3912.0
    }
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
    while s.turn < CAREER_LENGTH {
        take_action(&mut s, *possible_actions.choose(rng).unwrap(), deck, rng);
    }
    rating(&s.stats)
}

fn intelligently_run_career<R: Rng>(
    state: &mut State,
    deck: &[SupportCard],
    rng: &mut R,
) -> Vec<Action> {
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
        let best_action_to_take = possible_actions
            .iter()
            .max_by_key(|a| {
                let mut sum: f64 = 0.0;
                for _ in 0..1000 {
                    let mut s = state.clone();
                    take_action(&mut s, **a, deck, rng);
                    sum += random_rollout(s, deck, rng);
                }
                n64(sum)
            })
            .unwrap();
        actions.push(*best_action_to_take);
        take_action(state, *best_action_to_take, deck, rng);
    }
    actions
}

fn main() {
    let mut support_card_pool: Vec<SupportCard> =
        serde_json::from_slice(&std::fs::read("cards.json").unwrap()).unwrap();
    support_card_pool.retain(|x| x.rarity > 1 && x.limit_break == 4 && x.r#type <= 4);

    let mut rng = XorShiftRng::from_os_rng();

    let mut best_deck: Vec<SupportCard> = Vec::new();
    let mut best_rating: f64 = 0.0;
    let mut best_actions: Vec<Action> = Vec::new();
    let mut best_stats: [u16; 5] = [0; 5];

    // Try some decks
    for _ in 0..10 {
        support_card_pool.shuffle(&mut rng);
        let deck: &[SupportCard] = &support_card_pool[0..6];
        let mut state = State {
            energy: 100,
            mood: Mood::Normal,
            stats: [0, 0, 0, 0, 0],
            turn: 0,
            times_trained: [0, 0, 0, 0, 0],
            friendship: [0; 6],
            support_locations: [
                Stat::Speed,
                Stat::Speed,
                Stat::Speed,
                Stat::Speed,
                Stat::Speed,
            ],
        };
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
