#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[get("/")]
fn hello() -> &'static str {
  "Hello, world!"
}

fn main() {
  rocket::ignite().mount("/", routes![hello]).launch();
}


// #![allow(dead_code)]

// mod bitfield;
// mod base_converter;
// mod sequence;

// use self::bitfield::BitField;
// use self::base_converter::BaseConverter;
// use self::sequence::{Sequence, SupportedImageFormats};

// use std::fs::File;
// use std::path::Path;

// fn main() {
//   let alphabet = String::from("!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~");
//   let base_encoder = BaseConverter::new(alphabet);
//   let bf_encoder = BitField::new(vec![7, 7, 7]);

//   println!(
//     "{:?}, {:?}",
//     base_encoder.encode(bf_encoder.encode(vec![127, 127, 127]) as usize),
//     bf_encoder.encode(vec![127, 127, 127]),
//   );
//   println!(
//     "{:?}, {:?}",
//     base_encoder.encode(bf_encoder.encode(vec![0, 0, 0]) as usize),
//     bf_encoder.encode(vec![0, 0, 0]),
//   );
//   println!(
//     "{:?}, {:?}",
//     base_encoder.encode(bf_encoder.encode(vec![64, 64, 64]) as usize),
//     bf_encoder.encode(vec![64, 64, 64]),
//   );

//   let gif_buf = File::open(&Path::new("src/assets/g.gif")).unwrap();
//   let png_buf = File::open(&Path::new("src/assets/p.png")).unwrap();
//   let jpeg_buf = File::open(&Path::new("src/assets/pattern.jpg")).unwrap();
//   let x_buf = File::open(&Path::new("src/assets/x.png")).unwrap();

//   // Sequence::new(gif_buf, SupportedImageFormats::GIF, 8);
//   let p = Sequence::new(png_buf, SupportedImageFormats::PNG, 4, 100);
//   // Sequence::new(jpeg_buf, SupportedImageFormats::JPEG, 8);
//   //Sequence::new(x_buf, SupportedImageFormats::PNG, 8);

//   println!("{}", p.to_json());
// }
