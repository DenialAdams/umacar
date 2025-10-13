use crate::career::Stats;

pub fn rating(s: &Stats) -> f64 {
   // https://pastebin.com/qSfsjwGC from https://docs.google.com/spreadsheets/d/1AAZCVDMiCozDNKte-5JYSm7SWUeVJ0o87i1iDY1bnWM/edit?usp=sharing
   s.iter().copied().map(rate_stat).sum()
}

fn rate_stat(mut t: u16) -> f64 {
   const F: [f64; 25] = [
      0.5, 0.8, 1.0, 1.3, 1.6, 1.8, 2.1, 2.4, 2.6, 2.8, 2.9, 3.0, 3.1, 3.3, 3.4, 3.5, 3.9, 4.1, 4.2, 4.3, 5.2, 5.5,
      6.6, 6.8, 6.9,
   ];
   const M: [f64; 81] = [
      7.888, 8.0, 8.1, 8.3, 8.4, 8.5, 8.6, 8.8, 8.9, 9.0, 9.2, 9.3, 9.4, 9.6, 9.7, 9.8, 10.0, 10.1, 10.2, 10.3, 10.5,
      10.6, 10.7, 10.9, 11.0, 11.1, 11.3, 11.4, 11.5, 11.7, 11.8, 11.9, 12.1, 12.2, 12.3, 12.4, 12.6, 12.7, 12.8, 13.0,
      13.1, 13.2, 13.4, 13.5, 13.6, 13.8, 13.9, 14.0, 14.1, 14.3, 14.4, 14.5, 14.7, 14.8, 14.9, 15.1, 15.2, 15.3, 15.5,
      15.6, 15.7, 15.9, 16.0, 16.1, 16.2, 16.4, 16.5, 16.6, 16.8, 16.9, 17.0, 17.2, 17.3, 17.4, 17.6, 17.7, 17.8, 17.9,
      18.1, 18.2, 18.3,
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
