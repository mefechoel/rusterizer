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
