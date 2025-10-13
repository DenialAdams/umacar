use std::fmt::Display;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)] // When things settle down, can prune things here.
pub struct SupportCard {
   pub id: u32,
   #[serde(rename = "type")]
   pub r#type: u8,
   pub group: bool,
   pub rarity: u32,
   pub limit_break: u32,
   pub starting_stats: [u16; 5],
   pub type_stats: u32,
   pub stat_bonus: [u8; 6],
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
