use candle_core::Device;

/// Auto-detect best available compute device.
/// Tries CUDA first, then Metal, falls back to CPU.
pub fn get_device() -> Device {
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            eprintln!("Using CUDA device");
            return device;
        }
    }

    #[cfg(feature = "metal")]
    {
        if let Ok(device) = Device::new_metal(0) {
            eprintln!("Using Metal device");
            return device;
        }
    }

    eprintln!("Using CPU device");
    Device::Cpu
}
