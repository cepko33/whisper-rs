#![allow(clippy::uninlined_format_args)]

use hound::{SampleFormat, WavReader};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperSysContext, WhisperSysState};
use core::ffi::c_void;

fn parse_wav_file(path: &Path) -> Vec<i16> {
    let reader = WavReader::open(path).expect("failed to read file");

    if reader.spec().channels != 1 {
        panic!("expected mono audio file");
    }
    if reader.spec().sample_format != SampleFormat::Int {
        panic!("expected integer sample format");
    }
    if reader.spec().sample_rate != 16000 {
        panic!("expected 16KHz sample rate");
    }
    if reader.spec().bits_per_sample != 16 {
        panic!("expected 16 bits per sample");
    }

    reader
        .into_samples::<i16>()
        .map(|x| x.expect("sample"))
        .collect::<Vec<_>>()
}

fn main() {
    let arg1 = std::env::args()
        .nth(1)
        .expect("first argument should be path to WAV file");
    let audio_path = Path::new(&arg1);
    if !audio_path.exists() {
        panic!("audio file doesn't exist");
    }
    let arg2 = std::env::args()
        .nth(2)
        .expect("second argument should be path to Whisper model");
    let whisper_path = Path::new(&arg2);
    if !whisper_path.exists() {
        panic!("whisper file doesn't exist")
    }

    let original_samples = parse_wav_file(audio_path);
    let samples = whisper_rs::convert_integer_to_float_audio(&original_samples);

    let ctx = WhisperContext::new_with_params(
        &whisper_path.to_string_lossy(),
        WhisperContextParameters::default()
    ).expect("failed to open model");
    let mut state = ctx.create_state().expect("failed to create key");
    let mut params = FullParams::new(SamplingStrategy::default());
    params.set_initial_prompt("experience");
    //params.set_progress_callback_safe(|progress| println!("Progress callback: {}%", progress));
    //params.set_print_realtime(true);
    unsafe extern "C" fn seg_callback(
        c: *mut WhisperSysContext,
        s: *mut WhisperSysState,
        n: i32,
        v: *mut c_void
    ) {
        println!("{:?}", (*s));
    }

    unsafe {
        params.set_new_segment_callback(Some(seg_callback));
    }

    params.set_n_threads(8);
    params.set_split_on_word(true);
    params.set_token_timestamps(true);

    let st = std::time::Instant::now();
    state
        .full(params, &samples)
        .expect("failed to convert samples");
    let et = std::time::Instant::now();

    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");
    for i in 0..num_segments {
        let num_tokens = state
            .full_n_tokens(i)
            .expect("failted to get n tokens");

        for j in 0..num_tokens {
            let token = state
                .full_get_token_data(i, j)
                .expect("failed to get full token data");

            let token_text = state
                .full_get_token_text(i, j)
                .expect("failed to get full token text");

            println!("[{} - {}]: {}", token.t0, token.t1, token_text);

        }
        /*
        let segment = state
            .full_get_segment_text(i)
            .expect("failed to get segment");
        let start_timestamp = state
            .full_get_segment_t0(i)
            .expect("failed to get start timestamp");
        let end_timestamp = state
            .full_get_segment_t1(i)
            .expect("failed to get end timestamp");
        */
    }
    println!("took {}ms", (et - st).as_millis());
}
