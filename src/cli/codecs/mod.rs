use clap::Command;

use self::{
    avif::avif, farbfeld::farbfeld, jpeg::jpeg, jpeg_xl::jpeg_xl, mozjpeg::mozjpeg, oxipng::oxipng,
    png::png, ppm::ppm, qoi::qoi,
};

mod avif;
mod farbfeld;
mod jpeg;
mod jpeg_xl;
mod mozjpeg;
mod oxipng;
mod png;
mod ppm;
mod qoi;

impl Codecs for Command {
    fn codecs(self) -> Self {
        self.subcommands([
            avif(),
            farbfeld(),
            jpeg(),
            jpeg_xl(),
            mozjpeg(),
            oxipng(),
            png(),
            ppm(),
            qoi(),
        ])
    }
}

pub trait Codecs {
    fn codecs(self) -> Self;
}
