use crate::{sys, SoftmaxAlgo};

/// Specifies how the softmax input must be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoftmaxMode {
    /// The softmax operation is computed per image (N) across the dimensions C,H,W.
    Instance,
    /// The softmax operation is computed per spatial location (H,W) per image (N) across
    /// dimension C.
    Channel,
}

impl From<SoftmaxMode> for sys::cudnnSoftmaxMode_t {
    fn from(mode: SoftmaxMode) -> Self {
        match mode {
            SoftmaxMode::Channel => Self::CUDNN_SOFTMAX_MODE_CHANNEL,
            SoftmaxMode::Instance => Self::CUDNN_SOFTMAX_MODE_INSTANCE,
        }
    }
}
