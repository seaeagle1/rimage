use std::{io::Read, marker::PhantomData};

use webp::{AnimDecoder, DecodeAnimImage};
use zune_core::{bit_depth::BitDepth, bytestream::ZReaderTrait, colorspace::ColorSpace};
use zune_image::{errors::ImageErrors, frame::Frame, image::Image, traits::DecoderTrait};

pub struct WebPDecoder<R: Read> {
    inner: DecodeAnimImage,
    phantom: PhantomData<R>,
}

impl<R: Read> WebPDecoder<R> {
    pub fn try_new(mut source: R) -> Result<WebPDecoder<R>, ImageErrors> {
        let mut buf = Vec::new();
        source.read_to_end(&mut buf)?;

        let decoder = AnimDecoder::new(&buf);
        let img = decoder.decode().map_err(ImageErrors::ImageDecodeErrors)?;

        Ok(WebPDecoder {
            inner: img,
            phantom: PhantomData,
        })
    }
}

impl<R, T> DecoderTrait<T> for WebPDecoder<R>
where
    R: Read,
    T: ZReaderTrait,
{
    fn decode(&mut self) -> Result<Image, ImageErrors> {
        let (width, height) = <WebPDecoder<R> as DecoderTrait<T>>::dimensions(self).unwrap();
        let color = <WebPDecoder<R> as DecoderTrait<T>>::out_colorspace(self);

        let frames = self
            .inner
            .into_iter()
            .enumerate()
            .map(|(idx, frame)| {
                Frame::from_u8(frame.get_image(), color, idx, frame.get_time_ms() as usize)
            })
            .collect::<Vec<_>>();

        Ok(Image::new_frames(
            frames,
            BitDepth::Eight,
            width,
            height,
            color,
        ))
    }

    fn dimensions(&self) -> Option<(usize, usize)> {
        let frame = self.inner.get_frame(0).unwrap();

        Some((frame.width() as usize, frame.height() as usize))
    }

    fn out_colorspace(&self) -> ColorSpace {
        let frame = self.inner.get_frame(0).unwrap();

        match frame.get_layout() {
            webp::PixelLayout::Rgb => ColorSpace::RGB,
            webp::PixelLayout::Rgba => ColorSpace::RGBA,
        }
    }

    fn name(&self) -> &'static str {
        "webp"
    }
}

#[cfg(test)]
mod tests;
