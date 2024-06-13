use std::io::{ErrorKind, Write};
use zune_core::{bit_depth::BitDepth, bytestream::ZByteWriterTrait, colorspace::ColorSpace};
use zune_core::options::EncoderOptions;
use zune_image::{codecs::ImageFormat, errors::ImageErrors, image::Image, traits::EncoderTrait};

mod libjxl;
use libjxl::Encoder;

struct ZByteWriter<T: ZByteWriterTrait>
{
    writer: T,
}

impl<T: ZByteWriterTrait> Write for ZByteWriter<T>
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let rv = self.writer.write_bytes(buf);
        if rv.is_ok()
        { Ok(rv.unwrap()) }
        else { Err(std::io::Error::from(ErrorKind::Other)) }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let rv = self.writer.flush_bytes();
        if rv.is_ok()
        { Ok(()) }
        else { Err(std::io::Error::from(ErrorKind::Other)) }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let rv = self.writer.write_all_bytes(buf);
        if rv.is_ok()
        { Ok(())}
        else { Err(std::io::Error::from(ErrorKind::Other)) }
    }
}

/// A JpegXL encoder
#[derive(Default)]
pub struct LibJxlEncoder {
    options: EncoderOptions,
}

impl LibJxlEncoder {
    /// Create a new encoder
    pub fn new() -> LibJxlEncoder {
        LibJxlEncoder::default()
    }

    /// Create a new encoder with specified options
    pub fn new_with_options(options: EncoderOptions) -> LibJxlEncoder {
        LibJxlEncoder { options }
    }
}

impl EncoderTrait for LibJxlEncoder {
    fn name(&self) -> &'static str {
        "libjxl-encoder"
    }

    fn encode_inner<T: ZByteWriterTrait>(
        &mut self,
        image: &Image,
        sink: T,
    ) -> Result<usize, ImageErrors> {

            let mut encoder = Encoder::new();
            if self.options.quality() >= 100 {
                encoder = encoder.with_lossless();
            } else {
                encoder = encoder.with_lossy_distance(
                    Encoder::DistanceFromQuality(self.options.quality()));
            }

            let (width, height) = image.dimensions();
            let opt = self.options.set_width(width).set_height(height).set_colorspace(image.colorspace()).set_depth(image.depth());

            let write_sink = ZByteWriter{ writer: sink };
            let bytes_written = encoder.encode(write_sink, &image.flatten_to_u8()[0], opt).map_err(|e| {
                ImageErrors::EncodeErrors(zune_image::errors::ImgEncodeErrors::Generic(e.to_string()))
            })?;

            Ok(bytes_written as usize)
    }

    fn supported_colorspaces(&self) -> &'static [ColorSpace] {
        &[
            ColorSpace::Luma,
            ColorSpace::RGBA,
            ColorSpace::RGB,
        ]
    }

    fn format(&self) -> zune_image::codecs::ImageFormat {
        ImageFormat::JPEG_XL
    }

    fn supported_bit_depth(&self) -> &'static [BitDepth] {
        &[BitDepth::Eight, BitDepth::Sixteen, BitDepth::Float32]
    }

    fn default_depth(&self, depth: BitDepth) -> BitDepth {
        match depth {
            BitDepth::Float32 => BitDepth::Float32,
            BitDepth::Sixteen => BitDepth::Sixteen,
            _ => BitDepth::Eight,
        }
    }
}
