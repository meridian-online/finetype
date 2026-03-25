use candle_core::Device;

/// Auto-detect best available compute device.
/// Tries CUDA first, then Metal, falls back to CPU.
///
/// Returns `(device, device_name)` so the caller can log the device
/// at the appropriate time (before TUI alternate screen entry).
pub fn get_device() -> (Device, &'static str) {
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            return (device, "CUDA");
        }
    }

    #[cfg(feature = "metal")]
    {
        if let Ok(device) = Device::new_metal(0) {
            return (device, "Metal");
        }
    }

    (Device::Cpu, "CPU")
}
