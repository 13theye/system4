// Rendering-related types and state

pub mod render_state;

use nnpipe::renderers::SegmentGpu;

pub use render_state::RenderState;

pub type GpuSegmentBuffer = Vec<SegmentGpu>;
