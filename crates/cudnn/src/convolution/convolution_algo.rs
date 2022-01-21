use crate::{
    data_type::DataType,
    determinism::Determinism,
    error::{CudnnError, IntoResult},
    math_type::MathType,
    private, sys,
    tensor::{NCHWVectC8x32, NCHWVectC8x4, TensorFormat, NCHW, NHWC},
};

/// The best suited algorithm according to the layer specifications obtained through a heuristic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BestHeuristic<T> {
    raw: T,
    time: f32,
    workspace_size: usize,
    determinism: Determinism,
    math_type: MathType,
}

impl<T> BestHeuristic<T> {
    /// Returns the math type associated to the optimal algorithm.
    pub fn math_type(&self) -> MathType {
        self.math_type
    }

    /// Returns the workspace size associated to the optimal algorithm.
    pub fn workspace_size(&self) -> usize {
        self.workspace_size
    }

    /// Returns the determinism of the optimal algorithm.
    pub fn determinism(&self) -> Determinism {
        self.determinism
    }
}

// Convolution Forward Algorithms (as listed in the docs at:
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionFwdAlgo_t).
// Some of these algorithms can also be used in the backward path.

/// This algorithm expresses the convolution as a matrix product without actually explicitly
/// forming the matrix that holds the input tensor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplicitGemm;

/// This algorithm expresses convolution as a matrix product without actually explicitly forming
/// the matrix that holds the input tensor data, but still needs some memory workspace to
/// pre-compute some indices in order to facilitate the implicit construction of the matrix that
/// holds the input tensor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplicitPrecompGemm;

/// This algorithm expresses the convolution as an explicit matrix product. A significant memory
/// workspace is needed to store the matrix that holds the input tensor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gemm;

/// This algorithm expresses the convolution as a direct convolution (for example, without
/// implicitly or explicitly doing a matrix multiplication).
///
/// **Do note** that this is currently not implemented in cuDNN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Direct;

/// This algorithm uses the Fast-Fourier Transform approach to compute the convolution. A
/// significant memory workspace is needed to store intermediate results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fft;

/// This algorithm uses the Fast-Fourier Transform approach but splits the inputs into tiles.
/// A significant memory workspace is needed to store intermediate results, but less than
/// [`Fft`], for large size images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FftTiling;

/// This algorithm uses the Winograd Transform approach to compute the convolution. A reasonably
/// sized workspace is needed to store intermediate results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Winograd;

/// This algorithm uses the Winograd Transform approach to compute the convolution. A significant
/// workspace may be needed to store intermediate results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WinogradNonFused;

/// BestHeuristic for the forward convolution algorithm.
impl TryFrom<sys::cudnnConvolutionFwdAlgoPerf_t> for BestHeuristic<sys::cudnnConvolutionFwdAlgo_t> {
    type Error = CudnnError;

    fn try_from(raw: sys::cudnnConvolutionFwdAlgoPerf_t) -> Result<Self, Self::Error> {
        let sys::cudnnConvolutionFwdAlgoPerf_t {
            algo,
            status,
            time,
            memory,
            determinism,
            mathType,
            ..
        } = raw;
        status.into_result().map(|_| Self {
            raw: algo,
            time,
            workspace_size: memory,
            determinism: Determinism::from(determinism),
            math_type: mathType.into(),
        })
    }
}

// Convolution Backward Data Algorithms (as listed in
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionBwdDataAlgo_t).

/// This algorithm expresses the convolution as a sum of matrix products without actually explicitly
/// forming the matrix that holds the input tensor data. The sum is done using the atomic add
/// operation, thus the results are non-deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataAlgo0;

/// This algorithm expresses the convolution as a matrix product without actually explicitly forming
/// the matrix that holds the input tensor data. The results are deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataAlgo1;

/// BestHeuristic for the backward data convolution algorithm.
impl TryFrom<sys::cudnnConvolutionBwdDataAlgoPerf_t>
    for BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>
{
    type Error = CudnnError;

    fn try_from(raw: sys::cudnnConvolutionBwdDataAlgoPerf_t) -> Result<Self, Self::Error> {
        let sys::cudnnConvolutionBwdDataAlgoPerf_t {
            algo,
            status,
            time,
            memory,
            determinism,
            mathType,
            ..
        } = raw;
        status.into_result().map(|_| Self {
            raw: algo,
            time,
            workspace_size: memory,
            determinism: Determinism::from(determinism),
            math_type: mathType.into(),
        })
    }
}

// Convolution Backward Filter Algorithms (as listed in
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionBwdFilterAlgo_t).

/// This algorithm expresses the convolution as a sum of matrix products without actually explicitly
/// forming the matrix that holds the input tensor data. The sum is done using the atomic add
/// operation, thus the results are non-deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterAlgo0;

/// This algorithm expresses the convolution as a matrix product without actually explicitly forming
/// the matrix that holds the input tensor data. The results are deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterAlgo1;

/// This algorithm is similar to `FilterAlgo0` but uses some small workspace to pre-compute some
/// indices. The results are also non-deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterAlgo3;

/// BestHeuristic for the backward filter convolution algorithm.
impl TryFrom<sys::cudnnConvolutionBwdFilterAlgoPerf_t>
    for BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>
{
    type Error = CudnnError;

    fn try_from(raw: sys::cudnnConvolutionBwdFilterAlgoPerf_t) -> Result<Self, Self::Error> {
        let sys::cudnnConvolutionBwdFilterAlgoPerf_t {
            algo,
            status,
            time,
            memory,
            determinism,
            mathType,
            ..
        } = raw;
        status.into_result().map(|_| Self {
            raw: algo,
            time,
            workspace_size: memory,
            determinism: Determinism::from(determinism),
            math_type: mathType.into(),
        })
    }
}

pub trait ConvolutionFwdAlgo: private::Sealed {
    fn into_raw(&self) -> sys::cudnnConvolutionFwdAlgo_t;
}

pub trait ConvolutionBwdDataAlgo: private::Sealed {
    fn into_raw(&self) -> sys::cudnnConvolutionBwdDataAlgo_t;
}

pub trait ConvolutionBwdFilterAlgo: private::Sealed {
    fn into_raw(&self) -> sys::cudnnConvolutionBwdFilterAlgo_t;
}

macro_rules! impl_convolution_algo {
    ($trait:ident, $safe_type:ident, $raw_type:ty, $raw_variant:ident) => {
        impl $trait for $safe_type {
            fn into_raw(&self) -> $raw_type {
                <$raw_type>::$raw_variant
            }
        }
    };
}

pub trait SupportedConvFwd<
    InType,
    InFmt,
    FilterType,
    FilterFmt,
    CompType,
    OutType,
    OutFmt,
    const D: usize,
    const N: usize,
> where
    Self: ConvolutionFwdAlgo,
    InType: DataType,
    InFmt: TensorFormat,
    FilterType: DataType,
    FilterFmt: TensorFormat,
    CompType: DataType,
    OutType: DataType,
    OutFmt: TensorFormat,
{
}

macro_rules! impl_supported_conv_fwd {
    ($conv_fwd_algo:ty, $in_type:ty, $in_fmt:ty, $filter_type:ty, $filter_fmt:ty, $comp_type:ty, $out_type:ty, $out_fmt:ty, $dim_operands:expr, $dim_conv:expr) => {
        impl
            SupportedConvFwd<
                $in_type,
                $in_fmt,
                $filter_type,
                $filter_fmt,
                $comp_type,
                $out_type,
                $out_fmt,
                $dim_operands,
                $dim_conv,
            > for $conv_fwd_algo
        {
        }
    };
}

pub trait SupportedConvBwdData<
    FilterType,
    FilterFmt,
    OutGradType,
    OutGradFmt,
    CompType,
    InGradType,
    InGradFmt,
    const D: usize,
    const N: usize,
> where
    Self: ConvolutionBwdDataAlgo,
    FilterType: DataType,
    FilterFmt: TensorFormat,
    OutGradType: DataType,
    OutGradFmt: TensorFormat,
    CompType: DataType,
    InGradType: DataType,
    InGradFmt: TensorFormat,
{
}

macro_rules! impl_supported_conv_bwd_data {
    ($conv_bwd_data_algo:ty,  $filter_type:ty, $filter_fmt:ty,  $out_grad_type:ty, $out_grad_fmt:ty,$comp_type:ty, $in_grad_type:ty, $in_grad_fmt:ty, $dim_operands:expr, $dim_conv:expr) => {
        impl
            SupportedConvBwdData<
                $filter_type,
                $filter_fmt,
                $out_grad_type,
                $out_grad_fmt,
                $comp_type,
                $in_grad_type,
                $in_grad_fmt,
                $dim_operands,
                $dim_conv,
            > for $conv_bwd_data_algo
        {
        }
    };
}

pub trait SupportedConvBwdFilter<
    InType,
    InFmt,
    OutGradType,
    OutGradFmt,
    CompType,
    FilterGradType,
    FilterGradFmt,
    const D: usize,
    const N: usize,
> where
    Self: ConvolutionBwdFilterAlgo,
    InType: DataType,
    InFmt: TensorFormat,
    OutGradType: DataType,
    OutGradFmt: TensorFormat,
    CompType: DataType,
    FilterGradType: DataType,
    FilterGradFmt: TensorFormat,
{
}

macro_rules! impl_supported_conv_bwd_filter {
    ($conv_bwd_filter_algo:ty,  $in_type:ty, $in_fmt:ty,  $out_grad_type:ty, $out_grad_fmt:ty,$comp_type:ty, $filter_grad_type:ty, $filter_grad_fmt:ty, $dim_operands:expr, $dim_conv:expr) => {
        impl
            SupportedConvBwdFilter<
                $in_type,
                $in_fmt,
                $out_grad_type,
                $out_grad_fmt,
                $comp_type,
                $filter_grad_type,
                $filter_grad_fmt,
                $dim_operands,
                $dim_conv,
            > for $conv_bwd_filter_algo
        {
        }
    };
}

impl private::Sealed for Gemm {}
impl private::Sealed for ImplicitGemm {}
impl private::Sealed for ImplicitPrecompGemm {}
impl private::Sealed for Fft {}
impl private::Sealed for FftTiling {}
impl private::Sealed for Direct {}
impl private::Sealed for Winograd {}
impl private::Sealed for WinogradNonFused {}
impl private::Sealed for DataAlgo0 {}
impl private::Sealed for DataAlgo1 {}
impl private::Sealed for FilterAlgo0 {}
impl private::Sealed for FilterAlgo1 {}
impl private::Sealed for FilterAlgo3 {}
impl<T> private::Sealed for BestHeuristic<T> {}

#[rustfmt::skip]
mod impl_convolution_algo {
    use super::*;

    impl_convolution_algo!(ConvolutionFwdAlgo, ImplicitGemm, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_IMPLICIT_GEMM);
    impl_convolution_algo!(ConvolutionFwdAlgo, ImplicitPrecompGemm, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_IMPLICIT_PRECOMP_GEMM);
    impl_convolution_algo!(ConvolutionFwdAlgo, Gemm, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_GEMM);
    impl_convolution_algo!(ConvolutionFwdAlgo, Direct, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_DIRECT);
    impl_convolution_algo!(ConvolutionFwdAlgo, Fft, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_FFT);
    impl_convolution_algo!(ConvolutionFwdAlgo, FftTiling, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_FFT_TILING);
    impl_convolution_algo!(ConvolutionFwdAlgo, Winograd, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_WINOGRAD);
    impl_convolution_algo!(ConvolutionFwdAlgo, WinogradNonFused, sys::cudnnConvolutionFwdAlgo_t, CUDNN_CONVOLUTION_FWD_ALGO_IMPLICIT_GEMM);

    impl ConvolutionFwdAlgo for BestHeuristic<sys::cudnnConvolutionFwdAlgo_t> {
        fn into_raw(&self) -> sys::cudnnConvolutionFwdAlgo_t {
            self.raw
        }
    }

    impl_convolution_algo!(ConvolutionBwdDataAlgo, DataAlgo0, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_0);
    impl_convolution_algo!(ConvolutionBwdDataAlgo, DataAlgo1, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_1);
    impl_convolution_algo!(ConvolutionBwdDataAlgo, Fft, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_FFT);
    impl_convolution_algo!(ConvolutionBwdDataAlgo, FftTiling, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_FFT_TILING);
    impl_convolution_algo!(ConvolutionBwdDataAlgo, Winograd, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_WINOGRAD);
    impl_convolution_algo!(ConvolutionBwdDataAlgo, WinogradNonFused, sys::cudnnConvolutionBwdDataAlgo_t, CUDNN_CONVOLUTION_BWD_DATA_ALGO_WINOGRAD_NONFUSED);

    impl ConvolutionBwdDataAlgo for BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t> {
        fn into_raw(&self) -> sys::cudnnConvolutionBwdDataAlgo_t {
            self.raw
        }
    }

    impl_convolution_algo!(ConvolutionBwdFilterAlgo, FilterAlgo0, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_0);
    impl_convolution_algo!(ConvolutionBwdFilterAlgo, FilterAlgo1, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_1);
    impl_convolution_algo!(ConvolutionBwdFilterAlgo, FilterAlgo3, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_3);
    impl_convolution_algo!(ConvolutionBwdFilterAlgo, Fft, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_FFT);
    impl_convolution_algo!(ConvolutionBwdFilterAlgo, FftTiling, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_FFT_TILING);
    impl_convolution_algo!(ConvolutionBwdFilterAlgo, WinogradNonFused, sys::cudnnConvolutionBwdFilterAlgo_t, CUDNN_CONVOLUTION_BWD_FILTER_ALGO_WINOGRAD_NONFUSED);

    impl ConvolutionBwdFilterAlgo for BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t> {
        fn into_raw(&self) -> sys::cudnnConvolutionBwdFilterAlgo_t {
            self.raw
        }
    }
}

// Admissible configurations for the forward convolution (as specified in
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionForward)

#[rustfmt::skip]
mod supported_conv_fwd_impls {

    // The macro arguments are to be interpreted as follows, from left to right:
    // <algo. name> <input ty> <input fmt> <filter ty> <filter fmt> <comp. ty> <out ty> <out fmt>
    //
    // The last two integers are indicative of the convolution type.

    use super::*;
    /// ImplicitGemm supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(ImplicitGemm, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NHWC, f64, NCHW, f64, f64, NHWC, 4, 2);

    /// ImplicitPrecompGemm supported configurations for 2-d convolutions and filter format equal to
    /// NCHW.
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NHWC, f64, NCHW, f64, f64, NHWC, 4, 2);

    /// Gemm supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(Gemm, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(Gemm, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(Gemm, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(Gemm, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(Gemm, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(Gemm, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(Gemm, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(Gemm, f64, NHWC, f64, NCHW, f64, f64, NHWC, 4, 2);

    /// Fft supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(Fft, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);

    /// FftTiling supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(FftTiling, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(FftTiling, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);

    /// Winograd supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(Winograd, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(Winograd, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(Winograd, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(Winograd, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    
    /// WinogradNonFused supported configurations for 2-d convolutions and filter format equal to 
    /// NCHW.
    impl_supported_conv_fwd!(WinogradNonFused, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(WinogradNonFused, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(WinogradNonFused, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(WinogradNonFused, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    

    /// 2-d convolutions supported with NCHWVectC memory format.
    impl_supported_conv_fwd!(ImplicitGemm, i8, NCHWVectC8x4, i8, NCHWVectC8x4, i32, i8, NCHWVectC8x4, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, u8, NCHWVectC8x4, u8, NCHWVectC8x4, i32, u8, NCHWVectC8x4, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, i8, NCHWVectC8x32, i8, NCHWVectC8x32, i32, i8, NCHWVectC8x32, 4, 2);

    /// 2-d convolutions supported with NHWC memory format.
    impl_supported_conv_fwd!(ImplicitGemm, i8, NHWC, i8, NHWC, i32, i8, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, i8, NHWC, i8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, u8, NHWC, u8, NHWC, i32, u8, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, i8, NHWC, u8, NHWC, i32, u8, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, u8, NHWC, u8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitGemm, i8, NHWC, u8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NHWC, f64, NHWC, f64, f64, NHWC, 4, 2);

    /// ImplicitGemm supported configurations for 3-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(ImplicitGemm, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NCHW, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f32, NHWC, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NCHW, f64, NCHW, f64, f64, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitGemm, f64, NHWC, f64, NCHW, f64, f64, NHWC, 5, 3);

    /// ImplicitPrecompGemm supported configurations for 3-d convolutions and filter format equal to
    /// NCHW.
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NCHW, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NCHW, f64, NCHW, f64, f64, NHWC, 5, 3);
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f64, NHWC, f64, NCHW, f64, f64, NHWC, 5, 3);

    /// Fft supported configurations for 3-d convolutions and filter format equal to NCHW.
    impl_supported_conv_fwd!(FftTiling, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(FftTiling, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);

    /// Supported configurations for 3-d convolution with filter format equal to NHWC.
    impl_supported_conv_fwd!(ImplicitPrecompGemm, f32, NHWC, f32, NHWC, f32, f32, NHWC, 5, 3);

    /// BestHeuristic supported configurations. Its the set union of all those above.
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NCHWVectC8x4, i8, NCHWVectC8x4, i32, i8, NCHWVectC8x4, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, u8, NCHWVectC8x4, u8, NCHWVectC8x4, i32, u8, NCHWVectC8x4, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NCHWVectC8x32, i8, NCHWVectC8x32, i32, i8, NCHWVectC8x32, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NHWC, i8, NHWC, i32, i8, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NHWC, i8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, u8, NHWC, u8, NHWC, i32, u8, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NHWC, u8, NHWC, i32, u8, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, u8, NHWC, u8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, i8, NHWC, u8, NHWC, i32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NHWC, f64, NHWC, f64, f64, NHWC, 4, 2);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NHWC, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NHWC, 5, 3);
    impl_supported_conv_fwd!(BestHeuristic<sys::cudnnConvolutionFwdAlgo_t>, f32, NHWC, f32, NHWC, f32, f32, NHWC, 5, 3);
}

// Admissible configurations for the backward data convolution (as specified in
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionBackwardData).

#[rustfmt::skip]
mod supported_conv_bwd_data_impls {
    use super::*;

    // The macro arguments are to be interpreted as follows, from left to right:
    // <algo. name> <filter ty> <filter fmt> <diff. ty> <diff. fmt> <comp. ty> <in. diff ty> <in. diff fmt>
    //
    // The last two integers are indicative of the convolution type.

    /// DataAlgo0 supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(DataAlgo0, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo0, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo0, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo0, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    
    /// DataAlgo1 supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(DataAlgo1, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo1, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo1, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(DataAlgo1, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);

    /// Fft supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(Fft, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);

    /// FftTiling supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(FftTiling, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(FftTiling, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);

    /// Winograd supported configurations for 2-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(Winograd, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(Winograd, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    
    /// WinogradNonFused supported configurations for 2-d convolutions and filter format equal to 
    /// NCHW.
    impl_supported_conv_bwd_data!(WinogradNonFused, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(WinogradNonFused, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);

    /// DataAlgo0 supported configurations for 2-d convolutions and filter format equal to NHWC.
    impl_supported_conv_bwd_data!(DataAlgo0, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);

    /// DataAlgo1 supported configurations for 2-d convolutions and filter format equal to NHWC.
    impl_supported_conv_bwd_data!(DataAlgo1, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);

    /// DataAlgo0 supported configurations for 3-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(DataAlgo0, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_data!(DataAlgo0, f32, NCHW, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_bwd_data!(DataAlgo0, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_bwd_data!(DataAlgo0, f64, NCHW, f64, NCHW, f64, f64, NHWC, 5, 3);

    /// DataAlgo1 supported configurations for 3-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(DataAlgo1, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);

    /// FftTiling supported configurations for 3-d convolutions and filter format equal to NCHW.
    impl_supported_conv_bwd_data!(FftTiling, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_data!(FftTiling, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);

    /// DataAlgo1 supported configurations for 3-d convolutions and filter format equal to NHWC.
    impl_supported_conv_bwd_data!(DataAlgo1, f32, NHWC, f32, NHWC, f32, f32, NHWC, 5, 3);

    /// BestHeuristic supported configurations. Its the set union of all those above.
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NHWC, 4, 2);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f32, NCHW, f32, NCHW, f32, f32, NHWC, 5, 3);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_bwd_data!(BestHeuristic<sys::cudnnConvolutionBwdDataAlgo_t>, f64, NCHW, f64, NCHW, f64, f64, NHWC, 5, 3);
}

// Admissible configurations for the backward filter convolution  (as specified in
// https://docs.nvidia.com/deeplearning/cudnn/api/index.html#cudnnConvolutionBackwardFilter)

#[rustfmt::skip]
mod supported_conv_bwd_filter_impls {
    use super::*;

    // The macro arguments are to be interpreted as follows, from left to right:
    // <algo. name> <in. ty> <in. fmt> <diff. ty> <diff. fmt> <comp. ty> <filter diff ty> <filter diff fmt>
    //
    // The last two integers are indicative of the convolution type.

    /// FilterAlgo0 supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NHWC, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f64, NHWC, f64, NHWC, f64, f64, NCHW, 4, 2);

    /// FilterAlgo1 supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FilterAlgo1, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo1, f32, NHWC, f32, NHWC, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo1, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo1, f64, NHWC, f64, NHWC, f64, f64, NCHW, 4, 2);

    /// Fft supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(Fft, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);

    /// FftTiling supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FftTiling, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FftTiling, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);

    /// FilterAlgo3 supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FilterAlgo3, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo3, f32, NHWC, f32, NHWC, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo3, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo3, f64, NHWC, f64, NHWC, f64, f64, NCHW, 4, 2);

    /// WinogradNonFused supported configurations for 2-d convolutions and filter gradient format 
    /// equal to NCHW.
    impl_supported_conv_bwd_filter!(WinogradNonFused, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(WinogradNonFused, f32, NHWC, f32, NHWC, f32, f32, NCHW, 4, 2);

    /// FilterAlgo0 supported configurations for 2-d convolutions and filter gradient format equal 
    /// to NHWC.
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);


    /// FilterAlgo1 supported configurations for 2-d convolutions and filter gradient format equal
    /// to NHWC.
    impl_supported_conv_bwd_filter!(FilterAlgo1, f32, NHWC, f32, NCHW, f32, f32, NHWC, 4, 2);
    impl_supported_conv_bwd_filter!(FilterAlgo1, f32, NHWC, f32, NHWC, f32, f32, NHWC, 4, 2);

    /// FilterAlgo0 supported configurations for 3-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f32, NHWC, f32, NHWC, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(FilterAlgo0, f64, NHWC, f64, NHWC, f64, f64, NCHW, 5, 3);

    /// FilterAlgo3 supported configurations for 3-d convolutions and filter gradient format equal 
    /// to NCHW.
    impl_supported_conv_bwd_filter!(FilterAlgo3, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(FilterAlgo3, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);

    /// FilterAlgo1 supported configurations for 3-d convolutions and filter gradient format equal
    /// to NHWC.
    impl_supported_conv_bwd_filter!(FilterAlgo1, f32, NHWC, f32, NHWC, f32, f32, NCHW, 5, 3);

    /// BestHeuristic supported configurations. Its the set union of all those above.
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f32, NHWC, f32, NHWC, f32, f32, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f64, NHWC, f64, NHWC, f64, f64, NCHW, 4, 2);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f32, NHWC, f32, NCHW, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f32, NHWC, f32, NHWC, f32, f32, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f64, NHWC, f64, NCHW, f64, f64, NCHW, 5, 3);
    impl_supported_conv_bwd_filter!(BestHeuristic<sys::cudnnConvolutionBwdFilterAlgo_t>, f64, NHWC, f64, NHWC, f64, f64, NCHW, 5, 3);
}
