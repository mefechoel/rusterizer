#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

mod sequence;

use std::io;
use std::io::{Error, ErrorKind};
use rocket::Data;

use self::sequence::{
  Sequence,
  SupportedImageFormats,
};

#[post("/rasterize?<format>&<bit_depth>&<max_width>", data = "<data>")]
fn rasterize(
  data: Data,
  format: String,
  bit_depth: u32,
  max_width: u32,
) -> io::Result<String> {
  match SupportedImageFormats::from(format) {
    None => Err(Error::new(
      ErrorKind::InvalidInput,
      "Invalid image format",
    )),
    Some(mimetype) => {
      let seq = Sequence::new(
        data.open(),
        mimetype,
        bit_depth,
        max_width,
      );
      match seq {
        Err(err) => {
          println!("{:?}", err);
          Err(Error::new(
            ErrorKind::InvalidData,
            "Supplied image data could not be read",
          ))
        },
        Ok(seq) => {
          Ok(seq.stringify())
        },
      }
    }
  }
}

fn main() {
  rocket::ignite().mount("/", routes![rasterize]).launch();
}
