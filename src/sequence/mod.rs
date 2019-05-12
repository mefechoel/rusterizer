extern crate image;
extern crate rayon;

#[path = "../bitfield/mod.rs"]
mod bitfield;
#[path = "../base_converter/mod.rs"]
mod base_converter;
#[path = "../timestamp/mod.rs"]
mod timestamp;

use self::bitfield::BitField;
use self::base_converter::BaseConverter;
use self::timestamp::timestamp;

use image::{
  Frame,
  ImageError,
  Pixel,
  ImageBuffer,
  ColorType,
  GenericImageView,
  AnimationDecoder,
  ImageDecoder,
  RgbImage,
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
  frames: Vec<(Vec<u8>, u16)>,
  color_type: ColorType,
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
  ) -> Result<Sequence, ImageError> where T: std::io::Read {
    println!("starting...");
    let r_data_start = timestamp();
    let data = get_r_data(buffer, mimetype);
    println!("r_data: {}", timestamp() - r_data_start);

    match data {
      Err(err) => Err(err),
      Ok(data) => {
        let frames_start = timestamp();
        let (
          frames,
          min_delay,
          scaled_dimensions,
        ) = get_frames(data, max_width);
        println!("frames: {}", timestamp() - frames_start);
        let matrix_start = timestamp();
        let matrix = get_matrix(
          frames,
          scaled_dimensions,
          bit_depth,
        );
        println!("matrix: {}", timestamp() - matrix_start);
        let sequence = Sequence {
          min_delay,
          matrix,
          dimensions: scaled_dimensions,
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

fn gif_get_r_data<T>(
  decoder: image::gif::Decoder<T>,
) -> RasterizationData
  where T: std::io::Read
{
  let dimensions = to_dimensions(decoder.dimensions());
  let color_type = decoder.colortype();

  let wrapped_frames: Vec<Result<Frame, ImageError>> = decoder
    .into_frames()
    .collect();

  let frames = wrapped_frames
    .into_par_iter()
    .map(|wrapped_frame: Result<Frame, ImageError>| {
      let frame: Frame = wrapped_frame.unwrap();
      let delay = frame.delay().to_integer();
      let frame_buf = frame.into_buffer().into_vec();
      (frame_buf, delay)
    })
    .collect();

  RasterizationData {
    dimensions,
    frames,
    color_type,
  }
}

fn static_get_r_data<Dec>(decoder: Dec) -> RasterizationData
  where Dec: ImageDecoder
{
  let dimensions = to_dimensions(decoder.dimensions());
  let color_type = decoder.colortype();

  let img_vec = decoder
    .read_image()
    .unwrap();

  let min_delay: u16 = 1000;
  let frames = vec![(img_vec, min_delay)];

  RasterizationData {
    dimensions,
    frames,
    color_type,
  }
}

fn get_r_data<T>(
  buf: T,
  mimetype: SupportedImageFormats,
) -> Result<RasterizationData, ImageError>
  where T: std::io::Read
{
  match mimetype {
    SupportedImageFormats::GIF => {
      match image::gif::Decoder::new(buf) {
        Ok(gif) => Ok(gif_get_r_data(gif)),
        Err(err) => Err(err),
      }
    },
    SupportedImageFormats::PNG => {
      match image::png::PNGDecoder::new(buf) {
        Ok(png) => Ok(static_get_r_data(png)),
        Err(err) => Err(err),
      }
    },
    SupportedImageFormats::JPEG => {
      match image::jpeg::JPEGDecoder::new(buf) {
        Ok(jpeg) => Ok(static_get_r_data(jpeg)),
        Err(err) => Err(err),
      }
    },
  }
}

fn get_frames(
  data: RasterizationData,
  max_width: u32,
) -> (Vec<PixelVec>, u16, Dimensions) {
  let dimensions = data.dimensions;
  let color_type = data.color_type;
  let scaled_dimensions = scale_dimensions(
    dimensions,
    max_width,
  );

  let (frames, delays): (Vec<Vec<u8>>, Vec<u16>) = data
    .frames
    .into_par_iter()
    .unzip();

  let max = (2_u32.pow(16) - 1) as u16;
  let min_delay = delays
    .par_iter()
    .reduce(|| &max, |acc, delay| {
      if delay < &acc {
        delay
      } else {
        acc
      }
    });

  let pixel_frames: Vec<PixelVec> = frames
    .par_iter()
    .map(|frame: &Vec<u8>| frame
      .par_iter()
      .enumerate()
      .filter(|(i, _val)| {
        match color_type {
          ColorType::RGBA(8) => (i + 1) % 4 != 0,
          _ => true,
        }
      })
      .map(|(_i, val)| val + 0)
      .collect()
    )
    .map(|frame| {
      let frame_buf: RgbImage = RgbImage::from_vec(
        dimensions.0,
        dimensions.1,
        frame,
      ).unwrap();
      let buffer = resize(&frame_buf, scaled_dimensions);
      let pixels = buffer
        .pixels()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
      pixels
    })
    .collect();

  (pixel_frames, *min_delay, scaled_dimensions)
}

fn get_matrix(
  pixel_frames: Vec<PixelVec>,
  dimensions: Dimensions,
  bit_depth: u32,
) -> Vec<Vec<FramePixel>> {
  let loss = 2_u32.pow(COLOR_BIT_DEPTH - bit_depth);
  let bitfield = create_bitfield_encoder(bit_depth);
  let base_encoder = create_base_encoder();

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

  matrix
}

fn create_bitfield_encoder(bit_depth: u32) -> BitField {
  BitField::new(
    vec![bit_depth, bit_depth, bit_depth],
  )
}

fn create_base_encoder() -> BaseConverter {
  let alphabet = String::from(
    "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~",
  );
  BaseConverter::new(alphabet)
}
