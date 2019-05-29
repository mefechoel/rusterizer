extern crate image;

use image::ColorType;

pub type Dimensions = (u32, u32);

#[derive(Debug)]
pub struct RasterizationData {
  pub dimensions: Dimensions,
  pub frames: Vec<(Vec<u8>, u16)>,
  pub color_type: ColorType,
}
