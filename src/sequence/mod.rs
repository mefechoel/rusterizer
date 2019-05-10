#![allow(dead_code)]
extern crate image;
extern crate rayon;

#[path="../bitfield/mod.rs"]
mod bitfield;
#[path="../base_converter/mod.rs"]
mod base_converter;

use self::bitfield::BitField;
use self::base_converter::BaseConverter;

use std::fs::File;

use image::{
  Pixel,
  ImageBuffer,
  ColorType,
  GenericImageView,
  AnimationDecoder,
  ImageDecoder,
  RgbaImage,
  RgbImage,
  Rgba,
  Rgb,
  imageops,
};
// use rayon::prelude::*;
// use rayon::iter::IterBridge;

pub enum SupportedImageFormats {
  GIF,
  PNG,
  JPEG,
}

const IMG_WIDTH: u32 = 5;
const IMG_HEIGHT: u32 = 5;
const COLOR_BIT_DEPTH: u32 = 8;

type Col = [u8; 3];
type Dimensions = (u32, u32);
type PixelVec = Vec<Col>;
type EncodedColor = String;

#[derive(Debug, Clone)]
struct FramePixel {
  color: EncodedColor,
  duration: u16,
}

#[derive(Debug)]
struct RasterizationData {
  dimensions: Dimensions,
  frames: Vec<(PixelVec, u16)>,
}

#[derive(Debug)]
pub struct Sequence {
  dimensions: Dimensions,
  frames: Vec<Vec<FramePixel>>,
  min_delay: u16,
}

impl Sequence {
  pub fn new(
    buffer: File,
    mimetype: SupportedImageFormats,
    bit_depth: u32,
    max_width: u32,
  ) -> Sequence {
    let loss = 2_u32.pow(COLOR_BIT_DEPTH - bit_depth);

    let bitfield = BitField::new(vec![bit_depth, bit_depth, bit_depth]);
    let alphabet = String::from(
      "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~",
    );
    let base_encoder = BaseConverter::new(alphabet);

    let data = to_seq(buffer, mimetype, max_width);
    let dimensions = data.dimensions;
    let (pixel_frames, delays): (Vec<PixelVec>, Vec<u16>) = data
      .frames
      .into_iter()
      .unzip();

    let min_delay = delays.iter().fold(2_u16.pow(15), |acc, delay| {
      if delay < &acc {
        *delay
      } else {
        acc
      }
    });

    let frames: Vec<Vec<FramePixel>> = pixel_frames
      .iter()
      .map(|pixels| {
        let mut frame_stack: Vec<FramePixel> = Vec::new();
        for pixel in pixels {
          let resized_col = vec![
            (pixel[0] as u32) / loss,
            (pixel[1] as u32) / loss,
            (pixel[2] as u32) / loss,
          ];
          let bf_col = bitfield.encode(resized_col);
          let enc_col = base_encoder.encode(bf_col as usize);

          let prev = frame_stack.last_mut();
          match prev {
            Some(prev_pixel) => {
              if prev_pixel.color == enc_col {
                prev_pixel.duration += 1;
              } else {
                let next_pixel = FramePixel {
                  color: enc_col,
                  duration: 1,
                };
                frame_stack.push(next_pixel);
              }
            },
            None => {
              let next_pixel = FramePixel {
                color: enc_col,
                duration: 1,
              };
              frame_stack.push(next_pixel);
            },
          };
        }
        frame_stack
      })
      .collect();

    Sequence {
      min_delay,
      frames,
      dimensions,
    }
  }

  pub fn to_json(&self) -> String {
    let frame_stack_str = self
      .frames
      .iter()
      .fold(String::new(), |frame_acc, frame| {
        let frame_str = frame.iter().fold(String::new(), |pixel_acc, pixel| {
          let comma = if pixel_acc.len() > 0 {
            ","
          } else {
            ""
          };
          format!(
            "{}{}[\"{}\",{}]",
            pixel_acc,
            comma,
            pixel.color,
            pixel.duration,
          )
        });
        let comma = if frame_acc.len() > 0 {
          ","
        } else {
          ""
        };
        format!(
          "{}{}[{}]",
          frame_acc,
          comma,
          frame_str,
        )
      });
    
    format!("[{}]", frame_stack_str)
  }
}

fn rgba_to_rgb(pixel: &Rgba<u8>) -> [u8; 3] {
  [pixel.data[0], pixel.data[1], pixel.data[2]]
}

fn rgb_to_rgb(pixel: &Rgb<u8>) -> [u8; 3] {
  [pixel.data[0], pixel.data[1], pixel.data[2]]
}


fn resize<I: GenericImageView + 'static>(buf: &I, dimensions: Dimensions)
  -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
  I::Pixel: 'static,
  <I::Pixel as Pixel>::Subpixel: 'static,
{
  imageops::resize(
    buf,
    dimensions.0,
    dimensions.1,
    imageops::FilterType::Nearest,
  )
}

fn scale_dimensions(dimensions: Dimensions, width: u32) -> Dimensions {
  let ratio = width as f32 / dimensions.0 as f32;
  let height = (dimensions.1 as f32 * ratio) as u32;
  (width, height)
}

fn to_dimensions(dimensions: (u64, u64)) -> Dimensions {
  (dimensions.0 as u32, dimensions.1 as u32)
}

fn gif_to_seq(buf: File, max_width: u32) -> RasterizationData {
  let gif = image::gif::Decoder::new(buf).unwrap();
  let dimensions = to_dimensions(gif.dimensions());
  let scaled_dimensions = scale_dimensions(
    dimensions,
    max_width,  
  );

  let gif_frames = gif
    .into_frames()
    .map(|wrapped_frame| wrapped_frame.unwrap());

  let f = gif_frames
    .map(|frame: image::Frame| {
      let delay = frame.delay().to_integer();
      let frame_buf: RgbaImage = frame.into_buffer();
      let data = resize(&frame_buf, scaled_dimensions);
      let pixels = data.pixels().map(|p| rgba_to_rgb(p)).collect();
      (pixels, delay)
    });

  RasterizationData {
    dimensions: scaled_dimensions,
    frames: f.collect(),
  }
}

fn static_to_seq<Dec>(decoder: Dec, max_width: u32) -> RasterizationData
  where Dec: ImageDecoder
{
  let dimensions = to_dimensions(decoder.dimensions());
  let scaled_dimensions = scale_dimensions(
    dimensions,
    max_width,
  );
  let color_type = decoder.colortype();

  let img_vec = decoder
    .read_image()
    .unwrap()
    .iter()
    .enumerate()
    .filter(|(i, _val)| {
      match color_type {
        ColorType::RGBA(8) => (i + 1) % 4 != 0,
        _ => true,
      }
    })
    .map(|(_i, val)| val + 0)
    .collect();

  let frame_buf: RgbImage = RgbImage::from_vec(
    dimensions.0,
    dimensions.1,
    img_vec,
  ).unwrap();

  let data = resize(&frame_buf, scaled_dimensions);
  let pixels = data.pixels().map(|p| rgb_to_rgb(p)).collect();
  let min_delay: u16 = 1000;
  let frame = (pixels, min_delay);

  RasterizationData {
    dimensions: scaled_dimensions,
    frames: vec![frame],
  }
}

fn to_seq(buf: File, mimetype: SupportedImageFormats, max_width: u32) -> RasterizationData {
  match mimetype {
    SupportedImageFormats::GIF => gif_to_seq(buf, max_width),
    SupportedImageFormats::PNG => {
      let png = image::png::PNGDecoder::new(buf).unwrap();
      static_to_seq(png, max_width)
    },
    SupportedImageFormats::JPEG => {
      let jpeg = image::jpeg::JPEGDecoder::new(buf).unwrap();
      static_to_seq(jpeg, max_width)
    },
  }
}
