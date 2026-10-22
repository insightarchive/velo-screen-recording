# Local patch

Vendored copy of `windows-capture` 2.0.1 (MIT, see LICENCE) with one change to `src/encoder.rs`:
`VideoEncoder::pause()` / `resume()`. While paused, frames are dropped and the paused duration is
subtracted from later timestamps, so a paused recording has no frozen gap. Remove this copy and the
`[patch.crates-io]` entry in the workspace `Cargo.toml` if upstream adds equivalent support.
