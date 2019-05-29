#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Debug)]
pub struct BaseConverter {
  alphabet: HashMap<char, usize>,
  inv_alphabet: HashMap<usize, char>,
  base: usize,
}

impl BaseConverter {
  pub fn new(alphabet: String) -> BaseConverter {
    let mut alphabet_map: HashMap<char, usize> = HashMap::new();
    let mut inv_alphabet_map: HashMap<usize, char> = HashMap::new();
    for (i, c) in alphabet.chars().enumerate() {
      alphabet_map.insert(c, i);
      inv_alphabet_map.insert(i, c);
    }

    BaseConverter {
      alphabet: alphabet_map,
      inv_alphabet: inv_alphabet_map,
      base: alphabet.len(),
    }
  }

  pub fn encode(&self, input_num: usize) -> String {
    let mut num = input_num;
    let div = self.base;
    let mut str_repres = String::new();
    loop {
      let rest = num % div;
      str_repres = format!(
        "{}{}",
        self.inv_alphabet.get(&rest).unwrap(),
        str_repres,
      );
      num /= self.base;
      if num <= 0 {
        break;
      }
    }
    str_repres
  }

  pub fn decode(&self, str_repres: String) -> usize {
    let str_len = str_repres.len();
    str_repres
      .chars()
      .enumerate()
      .fold(0_usize, |acc, (i, character)| {
        let index = &str_len - 1 - i;
        let char_index = self.alphabet.get(&character).unwrap();
        acc + self.base.pow(index as u32) * char_index
      })
  }
}
