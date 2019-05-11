extern crate image;
extern crate rayon;

#[path="../bitfield/mod.rs"]
mod bitfield;
#[path="../base_converter/mod.rs"]
mod base_converter;

use self::bitfield::BitField;
use self::base_converter::BaseConverter;

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
use rayon::prelude::*;

pub enum SupportedImageFormats {
  GIF,
  PNG,
  JPEG,
}

impl SupportedImageFormats {
  pub fn from(mimetype: String) -> Option<SupportedImageFormats> {
    match mimetype.as_ref() {
      "png" => Some(SupportedImageFormats::PNG),
      "jpeg" => Some(SupportedImageFormats::JPEG),
      "gif" => Some(SupportedImageFormats::GIF),
      _ => None,
    }
  }
}

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
  matrix: Vec<Vec<FramePixel>>,
  min_delay: u16,
}

impl Sequence {
  pub fn new<T>(
    buffer: T,
    mimetype: SupportedImageFormats,
    bit_depth: u32,
    max_width: u32,
  ) -> Result<Sequence, image::ImageError> where T: std::io::Read {
    let loss = 2_u32.pow(COLOR_BIT_DEPTH - bit_depth);

    let bitfield = BitField::new(
      vec![bit_depth, bit_depth, bit_depth],
    );
    let alphabet = String::from(
      "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~",
    );
    let base_encoder = BaseConverter::new(alphabet);

    let data = to_seq(buffer, mimetype, max_width);

    match data {
      Err(err) => Err(err),
      Ok(data) => {
        let dimensions = data.dimensions;
        let (pixel_frames, delays): (Vec<PixelVec>, Vec<u16>) = data
          .frames
          .into_par_iter()
          .unzip();

        let max = 2_u16.pow(15);
        let min_delay = delays
          .par_iter()
          .reduce(|| &max, |acc, delay| {
            if delay < &acc {
              delay
            } else {
              acc
            }
          });

        let mut matrix: Vec<Vec<FramePixel>> = Vec::with_capacity(
          (dimensions.0 * dimensions.1) as usize,
        );
        for _ in 0..(dimensions.0 * dimensions.1) {
          matrix.push(Vec::<FramePixel>::new());
        }

        let frame_iter: Vec<Vec<FramePixel>> = pixel_frames
          .par_iter()
          .map(|pixels| pixels.par_iter().map(|pixel| {
            let resized_col = vec![
              (pixel[0] as u32) / loss,
              (pixel[1] as u32) / loss,
              (pixel[2] as u32) / loss,
            ];
            let bf_col = bitfield.encode(resized_col);
            let enc_col = base_encoder.encode(bf_col as usize);
            FramePixel {
              color: enc_col,
              duration: 1_u16,
            }
          }).collect())
          .collect();

        for frame in frame_iter {
          for (i, pixel) in frame.iter().enumerate() {
            let prev = matrix[i].last_mut();
            match prev {
              Some(prev_pixel) => {
                if prev_pixel.color == pixel.color {
                  prev_pixel.duration += 1;
                } else {
                  let next_pixel = FramePixel {
                    color: pixel.color.clone(),
                    duration: 1,
                  };
                  matrix[i].push(next_pixel);
                }
              },
              None => {
                let next_pixel = FramePixel {
                  color: pixel.color.clone(),
                  duration: 1,
                };
                matrix[i].push(next_pixel);
              },
            };
          }
        }

        let sequence = Sequence {
          min_delay: *min_delay,
          matrix,
          dimensions,
        };
        Ok(sequence)
      }
    }
  }

  pub fn stringify(&self) -> String {
    let frame_stack_str = self
      .matrix
      .iter()
      .fold(String::new(), |frame_acc, frame| {
        let frame_str = frame
          .iter()
          .fold(String::new(), |pixel_acc, pixel| {
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


fn resize<I: GenericImageView + 'static>(
  buf: &I,
  dimensions: Dimensions,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
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

fn scale_dimensions(
  dimensions: Dimensions,
  width: u32,
) -> Dimensions {
  let ratio = width as f32 / dimensions.0 as f32;
  let height = (dimensions.1 as f32 * ratio) as u32;
  (width, height)
}

fn to_dimensions(dimensions: (u64, u64)) -> Dimensions {
  (dimensions.0 as u32, dimensions.1 as u32)
}

fn gif_to_seq<T>(
  gif: image::gif::Decoder<T>,
  max_width: u32,
) -> RasterizationData
  where T: std::io::Read
{
  let dimensions = to_dimensions(gif.dimensions());
  let scaled_dimensions = scale_dimensions(
    dimensions,
    max_width,  
  );

  let f: Vec<Result<image::Frame, image::ImageError>> = gif
    .into_frames()
    .collect();

  let frames: Vec<(PixelVec, u16)> = f
    .par_iter()
    .map(|wrapped_frame: &Result<image::Frame, image::ImageError>| {
      let frame: &image::Frame = &wrapped_frame.as_ref().unwrap();
      let delay = frame.delay().to_integer();
      let frame_buf: &RgbaImage = frame.buffer();
      let data = resize(frame_buf, scaled_dimensions);
      let pixels = data.pixels().map(|p| rgba_to_rgb(p)).collect();
      (pixels, delay)
    })
    .collect();

  RasterizationData {
    dimensions: scaled_dimensions,
    frames,
  }
}

fn static_to_seq<Dec>(
  decoder: Dec,
  max_width: u32,
) -> RasterizationData
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
    .par_iter()
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

fn to_seq<T>(
  buf: T,
  mimetype: SupportedImageFormats,
  max_width: u32,
) -> Result<RasterizationData, image::ImageError>
  where T: std::io::Read
{
  match mimetype {
    SupportedImageFormats::GIF => {
      match image::gif::Decoder::new(buf) {
        Ok(gif) => Ok(gif_to_seq(gif, max_width)),
        Err(err) => Err(err),
      }
    },
    SupportedImageFormats::PNG => {
      match image::png::PNGDecoder::new(buf) {
        Ok(png) => Ok(static_to_seq(png, max_width)),
        Err(err) => Err(err),
      }
    },
    SupportedImageFormats::JPEG => {
      match image::jpeg::JPEGDecoder::new(buf) {
        Ok(jpeg) => Ok(static_to_seq(jpeg, max_width)),
        Err(err) => Err(err),
      }
    },
  }
}
