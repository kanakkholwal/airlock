//! Background transcription, always chunked: chunks are the unit of
//! progress, of yielding to dictation, and of retry.

use anyhow::Result;
use log::{debug, info, warn};
use std::time::Instant;
use transcribe_cpp::Transcript;

use super::{
    agreed_language, engine_error, post_process_transcription_text, real_time_factor, BatchContext,
    Transcription, TranscriptionManager,
};
use crate::audio_toolkit::chunking::Chunker;
use crate::managers::history::TimedSegment;
use crate::engine_supervisor::EngineError;
use crate::managers::audio::create_detector;

/// Tries per chunk when a dictation's cancel interrupts it.
const MAX_CHUNK_ATTEMPTS: usize = 3;

/// Audio handed to the chunker at a time: the first chunk is transcribed
/// without scoring the whole file first. Cuts match the whole-file plan (a
/// rare short blip at a feed boundary can nudge one within its pause).
const FEED_SAMPLES: usize = 60 * 16_000;

/// What a background transcription asks of whoever runs it.
pub trait ChunkControl {
    /// Called before each feed to the chunker and before each chunk is
    /// decoded. Blocks while dictation is using the engine; `false` stops the
    /// transcription.
    fn proceed(&self) -> bool;
    /// `done` of `total` samples are transcribed.
    fn progress(&self, done: usize, total: usize);
}

impl TranscriptionManager {
    /// Transcribe `audio` chunk by chunk with the selected model, one
    /// paragraph per chunk. `None` if `control` stopped it.
    pub fn transcribe_in_chunks(
        &self,
        audio: Vec<f32>,
        control: &dyn ChunkControl,
    ) -> Result<Option<Transcription>> {
        let started = Instant::now();
        let total = audio.len();
        if total == 0 {
            return Ok(Some(Transcription {
                text: String::new(),
                segments: vec![],
                language: None,
            }));
        }

        self.touch_activity();
        self.initiate_model_load(false);
        // Read once: the model can't change until the job ends.
        let BatchContext {
            settings,
            active_model,
            setup,
            ..
        } = self.batch_context(&audio)?;

        let backend = settings.vad_backend;
        let app_handle = self.app_handle.clone();
        let mut chunker = Chunker::new(Box::new(move || create_detector(backend, &app_handle)));
        let mut pieces = audio.chunks(FEED_SAMPLES);

        let mut transcripts: Vec<Transcript> = Vec::new();
        // Where each chunk starts in the file and how long it is, in milliseconds.
        let mut spans: Vec<(i64, i64)> = Vec::new();
        let mut done = 0;
        control.progress(done, total);
        loop {
            // Scoring waits out dictation too, and stops on cancel.
            if !control.proceed() {
                return Ok(None);
            }
            let (chunks, last) = match pieces.next() {
                Some(piece) => (chunker.push(piece), false),
                None => (chunker.finish(), true),
            };
            for chunk in chunks {
                let len = chunk.len();
                let mut attempts = 0;
                let transcript = loop {
                    if !control.proceed() {
                        return Ok(None);
                    }
                    attempts += 1;
                    // A fresh mark per chunk: a dictation's cancel stops only
                    // the chunk it caught, which is then tried again.
                    let mark = self.engine.cancel_mark();
                    match self.run_batch(mark, chunk.clone(), &setup.run_options, &active_model) {
                        Ok(transcript) => break transcript,
                        Err(EngineError::Cancelled) if attempts < MAX_CHUNK_ATTEMPTS => {
                            warn!("A background chunk was cancelled; trying it again");
                        }
                        Err(e) => return Err(engine_error(e)),
                    }
                };
                transcripts.push(transcript);
                spans.push((samples_to_ms(done), samples_to_ms(len)));
                done += len;
                control.progress(done, total);
            }
            if last {
                break;
            }
        }

        debug!("Transcribed {} chunks in the background", transcripts.len());
        let segments = build_segments(&transcripts, &spans);
        let language = setup.spoken_language(agreed_language(&transcripts).as_deref());
        let paragraphs: Vec<String> = transcripts
            .into_iter()
            .map(|t| {
                let output_language = setup.output_language(&settings, t.language);
                post_process_transcription_text(
                    t.text,
                    &settings,
                    setup.custom_words_prompted(),
                    &output_language,
                    &setup.languages,
                )
            })
            .filter(|p| !p.trim().is_empty())
            .collect();

        let elapsed_secs = started.elapsed().as_secs_f64();
        let audio_secs = total as f64 / 16_000.0;
        info!(
            "Background transcription of {:.1}s of audio in {} paragraphs took {:.1}s ({:.1}x real time)",
            audio_secs,
            paragraphs.len(),
            elapsed_secs,
            real_time_factor(audio_secs, elapsed_secs)
        );
        Ok(Some(Transcription {
            text: paragraphs.join("\n\n"),
            segments,
            language: Some(language),
        }))
    }
}

fn samples_to_ms(samples: usize) -> i64 {
    (samples as i64) * 1000 / 16_000
}

/// Puts every chunk's timed pieces on the file's own clock. A chunk whose model gave no usable
/// times still gets one piece covering the chunk, so nothing is left without a time.
fn build_segments(transcripts: &[Transcript], spans: &[(i64, i64)]) -> Vec<TimedSegment> {
    let mut out = Vec::new();
    for (t, (start, length)) in transcripts.iter().zip(spans) {
        let timed = t.segments.iter().any(|s| s.t1_ms > 0);
        let before = out.len();
        if timed {
            for s in t.segments.iter().filter(|s| !s.text.trim().is_empty()) {
                out.push(TimedSegment {
                    start_ms: start + s.t0_ms,
                    end_ms: start + s.t1_ms.max(s.t0_ms),
                    text: s.text.trim().to_string(),
                });
            }
        }
        if out.len() == before && !t.text.trim().is_empty() {
            out.push(TimedSegment {
                start_ms: *start,
                end_ms: start + length,
                text: t.text.trim().to_string(),
            });
        }
    }
    out
}

#[cfg(test)]
mod segment_tests {
    use super::*;
    use transcribe_cpp::Segment;

    fn piece(t0: i64, t1: i64, text: &str) -> Segment {
        Segment {
            t0_ms: t0,
            t1_ms: t1,
            text: text.into(),
            ..Segment::default()
        }
    }

    #[test]
    fn times_move_onto_the_files_clock() {
        let first = Transcript {
            text: "Hello there. Second.".into(),
            segments: vec![piece(0, 1500, "Hello there."), piece(1500, 2400, " Second.")],
            ..Transcript::default()
        };
        let second = Transcript {
            text: "Later.".into(),
            segments: vec![piece(0, 800, "Later.")],
            ..Transcript::default()
        };
        let out = build_segments(&[first, second], &[(0, 2400), (30_000, 800)]);
        assert_eq!(out.len(), 3);
        assert_eq!((out[1].start_ms, out[1].end_ms, out[1].text.as_str()), (1500, 2400, "Second."));
        assert_eq!((out[2].start_ms, out[2].end_ms), (30_000, 30_800));
    }

    #[test]
    fn a_chunk_without_times_still_gets_one_timed_piece() {
        let chunk = Transcript {
            text: "No times here.".into(),
            segments: vec![piece(0, 0, "No times here.")],
            ..Transcript::default()
        };
        let out = build_segments(&[chunk], &[(60_000, 5_000)]);
        assert_eq!(out, vec![TimedSegment { start_ms: 60_000, end_ms: 65_000, text: "No times here.".into() }]);
    }

    #[test]
    fn silent_chunks_add_nothing() {
        let quiet = Transcript::default();
        assert!(build_segments(&[quiet], &[(0, 1000)]).is_empty());
    }

    #[test]
    fn samples_convert_at_16_khz() {
        assert_eq!(samples_to_ms(16_000), 1000);
        assert_eq!(samples_to_ms(8_000), 500);
    }
}
