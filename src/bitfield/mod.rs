#[derive(Debug)]
struct BitFieldMask {
  mask: u32,
  shift: u32,
  max: u32,
}

#[derive(Debug)]
pub struct BitField {
  masks: Vec<BitFieldMask>,
}

impl BitField {
  pub fn new(shape: Vec<u32>) -> BitField {
    let shift_values: Vec<u32> = shape
      .clone()
      .into_iter()
      .enumerate()
      .map(|(i, _x)| {
        shape
          .clone()
          .into_iter()
          .enumerate()
          .filter(|(j, _y)| j < &i)
          .fold(0_u32, |acc, (_j, y)| acc + y)
      })
      .collect();

    let masks: Vec<BitFieldMask> = shape
      .into_iter()
      .enumerate()
      .map(|(i, bit_depth)| {
        let max = 2_u32.pow(bit_depth) - 1;
        let shift = shift_values[i];
        (max, shift)
      })
      .map(|(max, shift)| BitFieldMask {
        max,
        shift,
        mask: max << shift,
      })
      .collect();
    
    BitField {
      masks,
    }
  }

  pub fn encode(&self, data: Vec<u32>) -> u32 {
    if data.len() != self.masks.len() {
      panic!(
        "the input data must have length {}. found length {}",
        self.masks.len(),
        data.len(),
      );
    }

    self
      .masks
      .iter()
      .zip(data)
      .fold(0_u32, |acc, (m, val)| {
        acc | m.mask & (val << m.shift)
      })
  }

  pub fn decode(&self, bit_field: u32) -> Vec<u32> {
    self
      .masks
      .iter()
      .map(|m| (&bit_field & m.mask) >> m.shift)
      .collect()
  }
}
