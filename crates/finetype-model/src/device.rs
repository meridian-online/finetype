//! Shared compute-device selection for the crate's trainers and classifiers.

use candle_core::Device;

/// Return the best available compute device: CUDA, then Metal, then CPU.
pub(crate) fn get_device() -> Device {
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            return device;
        }
    }

    #[cfg(feature = "metal")]
    {
        if let Ok(device) = Device::new_metal(0) {
            return device;
        }
    }

    Device::Cpu
}
