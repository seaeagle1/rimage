#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::io::{Seek, SeekFrom, Write};
use std::os::raw::c_void;
use std::ptr::null;
use image::{ColorType, DynamicImage};
use thiserror::Error;

#[derive(Error,Debug)]
pub enum Error {
    #[error("JxlEncoder{0} failed")]
    JxlEncoder(String),
}

pub struct LibJxlEncoder {
    effort: i64,
    distance: f32,
}

impl LibJxlEncoder {
    pub fn new() -> Self
    {
        LibJxlEncoder{
            effort: 7,
            distance: 1f32,
        }
    }

    pub fn with_effort(mut self, value: i64) -> Self
    {
        self.effort = value;
        self
    }

    pub fn with_lossy_distance(mut self, value: f32) -> Self
    {
        self.distance = value;
        self
    }

    pub fn with_lossless(mut self) -> Self
    {
        self.distance = 0f32;
        self
    }

    pub fn DistanceFromQuality(quality: f32) -> f32 {
        unsafe { JxlEncoderDistanceFromQuality(quality) }
    }

    pub  fn encode<W: Write + Seek>(&self, mut output: W, imgdata: DynamicImage) -> Result<(), Error> {
        unsafe {
            let num_worker_threads = JxlThreadParallelRunnerDefaultNumWorkerThreads();
            let runner_opaque = JxlThreadParallelRunnerCreate(null(), num_worker_threads);
            let runner: JxlParallelRunner = None;

            let encoder = JxlEncoderCreate(null());
            if JxlEncoderSetParallelRunner(encoder, runner, runner_opaque)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetParallelRunner".to_string()));
            }

            let stream_start = output.stream_position().unwrap();
            let output_box = Box::new(OutputProcessorStruct {
                stream: Box::new(output),
                stream_start,
                buffer: None,
            });
            let output_processor = JxlEncoderOutputProcessor {
                opaque: Box::into_raw(output_box) as *mut c_void,
                get_buffer: Some(outputGetBuffer),
                release_buffer: Some(outputReleaseBuffer),
                seek: Some(outputSeek),
                set_finalized_position: Some(outputSetFinal),
            };
            if JxlEncoderSetOutputProcessor(encoder, output_processor)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetOutputProcessor".to_string()));
            }

            let settings = JxlEncoderFrameSettingsCreate(encoder, null());
            if JxlEncoderFrameSettingsSetOption(settings,
                                                JxlEncoderFrameSettingId_JXL_ENC_FRAME_SETTING_EFFORT,
                                                self.effort) != JxlEncoderStatus_JXL_ENC_SUCCESS
                ||
                JxlEncoderFrameSettingsSetOption(settings,
                                                 JxlEncoderFrameSettingId_JXL_ENC_FRAME_SETTING_BUFFERING,
                                                 2) != JxlEncoderStatus_JXL_ENC_SUCCESS
            {
                return Err(Error::JxlEncoder("FrameSettingsSetOption".to_string()));
            }
            if JxlEncoderSetFrameDistance(settings, self.distance.clone())
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetFrameDistance".to_string()));
            }
            let frame_bit_depth = Box::new(JxlBitDepth {
                type_: JxlBitDepthType_JXL_BIT_DEPTH_FROM_PIXEL_FORMAT,
                bits_per_sample: (imgdata.color().bits_per_pixel()
                    / imgdata.color().channel_count() as u16) as u32,
                exponent_bits_per_sample:
                if imgdata.color() == ColorType::Rgb32F || imgdata.color() == ColorType::Rgba32F { 8 } else { 0 },
            });
            if JxlEncoderSetFrameBitDepth(settings, &*frame_bit_depth)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetFrameBitDepth".to_string()));
            }
            if imgdata.color().has_alpha() {
                if JxlEncoderSetExtraChannelDistance(settings, 0, self.distance.clone())
                    != JxlEncoderStatus_JXL_ENC_SUCCESS {
                    return Err(Error::JxlEncoder("SetExtraChannelDistance".to_string()));
                }
            }
            if self.distance == 0f32 {
                if JxlEncoderSetFrameLossless(settings, 1)
                    != JxlEncoderStatus_JXL_ENC_SUCCESS {
                    return Err(Error::JxlEncoder("SetFrameLossless".to_string()));
                }
            }

            let n_color_channels = match imgdata.color() {
                ColorType::L8 | ColorType::L16 => 1,
                ColorType::La8 | ColorType::La16 => 1,
                _ => 3
            };

            let n_alpha_channels = match imgdata.color() {
                ColorType::La8 | ColorType::La16 => 1,
                ColorType::Rgba8 | ColorType::Rgba16 => 1,
                ColorType::Rgba32F => 1,
                _ => 0
            };
            let basic_info = Box::new(JxlBasicInfo {
                have_container: 1,
                xsize: imgdata.width(),
                ysize: imgdata.height(),
                bits_per_sample: (imgdata.color().bits_per_pixel()
                    / imgdata.color().channel_count() as u16) as u32,
                exponent_bits_per_sample:
                if imgdata.color() == ColorType::Rgb32F || imgdata.color() == ColorType::Rgba32F { 8 } else { 0 },
                intensity_target: 0.0,
                min_nits: 0.0,
                relative_to_max_display: 0,
                linear_below: 0.0,
                uses_original_profile:
                if self.distance == 0f32 { 1 } else { 0 },
                have_preview: 0,
                have_animation: 0,
                orientation: 1,
                num_color_channels: n_color_channels,
                num_extra_channels: n_alpha_channels,
                alpha_bits:
                if imgdata.color().has_alpha() {
                    (imgdata.color().bits_per_pixel() / imgdata.color().channel_count() as u16) as u32
                } else { 0 },
                alpha_exponent_bits: if imgdata.color() == ColorType::Rgba32F { 8 } else { 0 },
                alpha_premultiplied: 0,
                preview: JxlPreviewHeader { xsize: 0, ysize: 0 },
                animation: JxlAnimationHeader {
                    tps_numerator: 0,
                    tps_denominator: 0,
                    num_loops: 0,
                    have_timecodes: 0,
                },
                intrinsic_xsize: imgdata.width(),
                intrinsic_ysize: imgdata.height(),
                padding: [0; 100],
            });
            if JxlEncoderSetBasicInfo(encoder, &*basic_info)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetBasicInfo".to_string()));
            }

            if JxlEncoderUseContainer(encoder, 1)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("UseContainer".to_string()));
            }

            if JxlEncoderSetCodestreamLevel(encoder, 5)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetCodestreamLevel".to_string()));
            }

            let mut default_color_encoding = Box::new(JxlColorEncoding {
                color_space: 0,
                white_point: 0,
                white_point_xy: [0.0; 2],
                primaries: 0,
                primaries_red_xy: [0.0; 2],
                primaries_green_xy: [0.0; 2],
                primaries_blue_xy: [0.0; 2],
                transfer_function: 0,
                gamma: 0.0,
                rendering_intent: 0,
            });
            JxlColorEncodingSetToSRGB(&mut *default_color_encoding, 0);
            if JxlEncoderSetColorEncoding(encoder, &*default_color_encoding)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("SetColorEncoding".to_string()));
            }

            // encode frame
            let pixelformat = Box::new(JxlPixelFormat {
                num_channels: imgdata.color().channel_count() as u32,
                data_type:
                match imgdata.color() {
                    ColorType::L8 | ColorType::La8 => JxlDataType_JXL_TYPE_UINT8,
                    ColorType::Rgb8 | ColorType::Rgba8 => JxlDataType_JXL_TYPE_UINT8,
                    ColorType::L16 | ColorType::La16 => JxlDataType_JXL_TYPE_UINT16,
                    ColorType::Rgb16 | ColorType::Rgba16 => JxlDataType_JXL_TYPE_UINT16,
                    ColorType::Rgb32F | ColorType::Rgba32F => JxlDataType_JXL_TYPE_FLOAT,
                    _ => todo!()
                },
                endianness: JxlEndianness_JXL_NATIVE_ENDIAN,
                align: 0,
            });
            if JxlEncoderAddImageFrame(settings, &*pixelformat,
                                       imgdata.as_bytes().as_ptr() as *const c_void, imgdata.as_bytes().len())
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("AddImageFrame".to_string()));
            }

            JxlEncoderCloseInput(encoder);
            if JxlEncoderFlushInput(encoder)
                != JxlEncoderStatus_JXL_ENC_SUCCESS {
                return Err(Error::JxlEncoder("FlushInput".to_string()));
            }

            JxlEncoderDestroy(encoder);
            JxlThreadParallelRunnerDestroy(runner_opaque);
            let _pointer_for_garbage = Box::from_raw(output_processor.opaque);
        }
        Ok(())
    }
}

trait WriteAndSeek: Write + Seek {}
impl<T: Write+Seek> WriteAndSeek for T {}

struct OutputProcessorStruct<'a>
{
    stream: Box<dyn WriteAndSeek + 'a>,
    stream_start: u64,
    buffer: Option<Vec<u8>>,
}

unsafe extern "C" fn outputGetBuffer
    (opaque: *mut c_void, size: *mut usize) -> *mut c_void {
    let s = &mut *(opaque as *mut OutputProcessorStruct);
    let req_size = (*size).clone();

    if s.buffer.is_none() {
        s.buffer = Some(vec![0; req_size]);
    }
    let b = s.buffer.as_mut().unwrap();
    if b.capacity() < req_size {
        b.resize(req_size, 0);
    }

    *size = b.capacity();

    b.as_mut_ptr() as *mut c_void
}

unsafe extern "C" fn outputReleaseBuffer
    (opaque: *mut c_void, written_bytes: usize) {
    let s = &mut *(opaque as *mut OutputProcessorStruct);
    let b = s.buffer.as_ref().unwrap();

    s.stream.write_all(&b[0..written_bytes] ).expect("Writing JXL output failed");
}

unsafe extern "C" fn outputSeek(opaque: *mut c_void, position: u64) {
    let s = &mut *(opaque as *mut OutputProcessorStruct);
    s.stream.seek(SeekFrom::Start(&s.stream_start + position)).expect("Seeking JXL output failed");
}

unsafe extern "C" fn outputSetFinal(_opaque: *mut c_void, _finalized_position: u64) {}