use clap::Command;
use indoc::indoc;

use crate::cli::common::CommonArgs;

pub fn jpeg_xl() -> Command {
    Command::new("jpeg_xl")
        .alias("jxl")
        .about("Encode images into JpegXL format. (Big but Lossless)")
        .long_about(indoc! {r#"Encode images into jpeg xl format.

        Only supports lossless encoding"#})
        .common_args()
}

pub fn libjxl() -> Command {
    Command::new("libjxl")
        .about("Encode images into JpegXL format using reference encoder.")
        .long_about(indoc! {r#"Encode images into jpeg xl format."#})
        .common_args()
}