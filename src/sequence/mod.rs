extern crate image;
extern crate rayon;

#[path = "../bitfield/mod.rs"]
mod bitfield;
#[path = "../base_converter/mod.rs"]
mod base_converter;
#[path = "../timestamp/mod.rs"]
mod timestamp;
mod image_formats;
mod frame_pixel;
mod rasterization_data;

use self::bitfield::BitField;
use self::base_converter::BaseConverter;
use self::timestamp::timestamp;
use self::frame_pixel::FramePixel;
use self::rasterization_data::{RasterizationData, Dimensions};
pub use self::image_formats::SupportedImageFormats;

use std::io::{Error, ErrorKind};
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

const COLOR_BIT_DEPTH: u32 = 8;

type Col = [u8; 3];
type PixelVec = Vec<Col>;
type Matrix = Vec<Vec<FramePixel>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Sequence {
  step_length: u16,
  length: usize,
  duration: usize,
  width: u32,
  height: u32,
  matrices: Vec<String>,
  num_chunks: usize,
  max_chunk_size: usize,
  bit_depth: u32,
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
    let data = get_r_data(buffer, mimetype)?;
    println!("r_data: {}", timestamp() - r_data_start);

    let frames_start = timestamp();
    let length = data.frames.len();
    let (
      frames,
      min_delay,
      scaled_dimensions,
    ) = get_frames(data, max_width)?;
    println!("frames: {}", timestamp() - frames_start);

    let matrix_start = timestamp();
    let matrix = get_matrix(
      frames,
      scaled_dimensions,
      bit_depth,
    );
    println!("matrix: {}", timestamp() - matrix_start);

    let stringify_start = timestamp();
    let num_chunks: usize = 1;
    let mut matrices: Vec<String> = Vec::<String>::with_capacity(
      num_chunks,
    );
    let stringified_matrix = stringify_matrix(matrix);
    matrices.push(stringified_matrix);
    println!("stringify: {}", timestamp() - stringify_start);

    let sequence = Sequence {
      length,
      step_length: min_delay,
      duration: (min_delay as usize) * length,
      matrices,
      width: scaled_dimensions.0,
      height: scaled_dimensions.1,
      num_chunks,
      max_chunk_size: length,
      bit_depth,
    };
    Ok(sequence)
  }
}

fn stringify_matrix(matrix: Matrix) -> String {
  let mut stack = String::new();
  stack.push('[');
  let frame_strings: Vec<String> = matrix
    .par_iter()
    .map(|frame| {
      let mut frame_stack = String::new();
      frame_stack.push('[');
      for (j, pixel) in frame.iter().enumerate() {
        if j != 0 {
          frame_stack.push(',');
        }
        frame_stack.push_str("[\"");
        frame_stack.push_str(&pixel.color);
        frame_stack.push_str("\",");
        frame_stack.push_str(&pixel.duration.to_string());
        frame_stack.push(']');
      }
      frame_stack.push(']');
      frame_stack
    })
    .collect();

  for (i, frame) in frame_strings.iter().enumerate() {
    if i != 0 {
      stack.push(',');
    }
    stack.push_str(&frame);
  }
  stack.push(']');
  stack
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
    imageops::FilterType::Lanczos3,
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
) -> Result<RasterizationData, ImageError>
  where T: std::io::Read
{
  let dimensions = to_dimensions(decoder.dimensions());
  let color_type = decoder.colortype();

  let wrapped_frames: Vec<Result<Frame, ImageError>> = decoder
    .into_frames()
    .collect();

  let frames: Result<Vec<(Vec<u8>, u16)>, ImageError> = wrapped_frames
    .into_par_iter()
    .map(|wrapped_frame: Result<Frame, ImageError>| {
      let frame: Frame = wrapped_frame?;
      let delay = frame.delay().to_integer();
      let frame_buf = frame.into_buffer().into_vec();
      Ok((frame_buf, delay))
    })
    .collect();

  Ok(RasterizationData {
    dimensions,
    frames: frames?,
    color_type,
  })
}

fn static_get_r_data<Dec>(
  decoder: Dec,
) -> Result<RasterizationData, ImageError>
  where Dec: ImageDecoder
{
  let dimensions = to_dimensions(decoder.dimensions());
  let color_type = decoder.colortype();

  let img_vec = decoder.read_image()?;

  let min_delay: u16 = 1000;
  let frames = vec![(img_vec, min_delay)];

  Ok(RasterizationData {
    dimensions,
    frames,
    color_type,
  })
}

fn get_r_data<T>(
  buf: T,
  mimetype: SupportedImageFormats,
) -> Result<RasterizationData, ImageError>
  where T: std::io::Read
{
  match mimetype {
    SupportedImageFormats::GIF => {
      let decoder = image::gif::Decoder::new(buf)?;
      gif_get_r_data(decoder)
    },
    SupportedImageFormats::PNG => {
      let decoder = image::png::PNGDecoder::new(buf)?;
      static_get_r_data(decoder)
    },
    SupportedImageFormats::JPEG => {
      let decoder = image::jpeg::JPEGDecoder::new(buf)?;
      static_get_r_data(decoder)
    },
  }
}

fn get_frames(
  data: RasterizationData,
  max_width: u32,
) -> Result<(Vec<PixelVec>, u16, Dimensions), ImageError> {
  let dimensions = data.dimensions;
  let color_type = data.color_type;
  let scaled_dimensions = scale_dimensions(
    dimensions,
    max_width,
  );

  let (frames, delays): (Vec<Vec<u8>>, Vec<u16>) = data
    .frames
    .into_iter()
    .unzip();

  let max = (2_u32.pow(16) - 1) as u16;
  let min_delay = delays
    .into_iter()
    .fold(max, |acc, delay| {
      if delay < acc {
        delay
      } else {
        acc
      }
    });

  let pixel_frames: Result<Vec<PixelVec>, ImageError> = frames
    .into_par_iter()
    .map(|frame: Vec<u8>| frame
      .into_iter()
      .enumerate()
      .filter_map(|(i, val)| {
        match color_type {
          ColorType::RGBA(8) => {
            if (i + 1) % 4 != 0 {
              Some(val)
            } else {
              None
            }
          },
          _ => Some(val),
        }
      })
      .collect()
    )
    .map(|frame| {
      let frame_buf: Option<RgbImage> = RgbImage::from_vec(
        dimensions.0,
        dimensions.1,
        frame,
      );
      match frame_buf {
        Some(frame_buf) => {
          let buffer = resize(&frame_buf, scaled_dimensions);
          let pixels = buffer
            .pixels()
            .map(|p| [p[0], p[1], p[2]])
            .collect();
          Ok(pixels)
        },
        None => Err(ImageError::from(Error::new(
          ErrorKind::InvalidData,
          String::from("No frame found"),
        ))),
      }
    })
    .collect();

  Ok((
    pixel_frames?,
    min_delay,
    scaled_dimensions,
  ))
}

fn get_matrix(
  pixel_frames: Vec<PixelVec>,
  dimensions: Dimensions,
  bit_depth: u32,
) -> Matrix {
  let loss = 2_u32.pow(COLOR_BIT_DEPTH - bit_depth);
  let bitfield = create_bitfield_encoder(bit_depth);
  let base_encoder = create_base_encoder();

  let mut matrix: Matrix = Vec::with_capacity(
    (dimensions.0 * dimensions.1) as usize,
  );
  for _ in 0..(dimensions.0 * dimensions.1) {
    matrix.push(Vec::<FramePixel>::new());
  }

  let frame_iter: Matrix = pixel_frames
    .par_iter()
    .map(|pixels| pixels.iter().map(|pixel| {
      let resized_col = vec![
        u32::from(pixel[0]) / loss,
        u32::from(pixel[1]) / loss,
        u32::from(pixel[2]) / loss,
      ];
      let bf_col = bitfield.encode(&resized_col);
      let enc_col = base_encoder.encode(bf_col as usize);
      FramePixel {
        color: enc_col,
        duration: 1_u16,
      }
    }).collect())
    .collect();

  for frame in frame_iter {
    for (i, pixel) in frame.into_iter().enumerate() {
      let prev = matrix[i].last_mut();
      match prev {
        Some(prev_pixel) => {
          if prev_pixel.color == pixel.color {
            prev_pixel.duration += 1;
          } else {
            matrix[i].push(pixel);
          }
        },
        None => matrix[i].push(pixel),
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
