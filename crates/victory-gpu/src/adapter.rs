//! Which adapter the run gets, and a sentence describing it.
//!
//! `D3D12CreateDevice(None, …)` asks Windows for "the default adapter", and on
//! a machine with two GPUs that is the one driving the desktop - the integrated
//! one - not the fast one. The title then runs on the wrong card: at a third of
//! the frame rate if it starts at all, and on some laptops it fails the Shader
//! Model 6.6 check while the discrete card sits idle.
//!
//! This walks DXGI's performance order instead (`EnumAdapterByGpuPreference`),
//! skips software, remote and UMA adapters, and takes the first that answers
//! D3D12 + Shader Model 6.6 (and mesh shaders, when the title needs them).
//! When nothing answers it says why in words the user can act on.

use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::Interface;

/// `D3D12_ERROR_INVALID_REDIST`: the Agility SDK is not beside the executable.
const INVALID_REDIST: u32 = 0x887E_0003;

/// The device to run on: the fastest adapter that supports what the title needs.
///
/// `require_mesh` is the Series X requirement (mesh shaders). A device whose
/// memory architecture cannot be read is kept as a fallback rather than
/// rejected: some drivers refuse `ARCHITECTURE1` and still work.
pub fn hardware_device(require_mesh: bool) -> windows::core::Result<ID3D12Device> {
    // SAFETY: DXGI owns the enumerated interfaces, every feature structure is
    // initialised before the call that fills it, and each identifier is paired
    // with its own structure.
    unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
        let performance = factory.cast::<IDXGIFactory6>().ok();
        // (rank, device): rank 1 is an adapter whose kind is unknown, 0 a UMA
        // one. Both are only used when nothing discrete answers.
        let mut fallback: Option<(u8, ID3D12Device)> = None;
        for index in 0.. {
            let adapter = match &performance {
                Some(factory) => factory.EnumAdapterByGpuPreference::<IDXGIAdapter1>(
                    index,
                    DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,
                ),
                None => factory.EnumAdapters1(index),
            };
            let Ok(adapter) = adapter else { break };
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            let software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
            let remote = (desc.Flags & DXGI_ADAPTER_FLAG_REMOTE.0 as u32) != 0;
            if software || remote {
                continue;
            }

            let mut device: Option<ID3D12Device> = None;
            if let Err(error) = D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut device) {
                // The Agility SDK is per process, not per adapter: report it
                // instead of quietly trying the next card.
                if error.code().0 as u32 == INVALID_REDIST {
                    return Err(windows::core::Error::new(
                        error.code(),
                        "D3D12 Agility SDK failed to load: D3D12Core.dll is missing beside \
                         victory.exe (a build without --features agility-sdk does not need it)",
                    ));
                }
                continue;
            }
            let Some(device) = device else { continue };

            let mut model = D3D12_FEATURE_DATA_SHADER_MODEL {
                HighestShaderModel: D3D_SHADER_MODEL_6_6,
            };
            if !has(&device, D3D12_FEATURE_SHADER_MODEL, &mut model)
                || model.HighestShaderModel.0 < D3D_SHADER_MODEL_6_6.0
            {
                continue;
            }
            if require_mesh {
                let mut options = D3D12_FEATURE_DATA_D3D12_OPTIONS7::default();
                if !has(&device, D3D12_FEATURE_D3D12_OPTIONS7, &mut options)
                    || options.MeshShaderTier == D3D12_MESH_SHADER_TIER_NOT_SUPPORTED
                {
                    continue;
                }
            }

            let mut architecture = D3D12_FEATURE_DATA_ARCHITECTURE1::default();
            if !has(&device, D3D12_FEATURE_ARCHITECTURE1, &mut architecture) {
                if fallback.as_ref().is_none_or(|(rank, _)| *rank < 1) {
                    fallback = Some((1, device));
                }
            } else if !architecture.UMA.as_bool() {
                // A discrete card in DXGI's performance order: the answer.
                return Ok(device);
            } else if fallback.is_none() {
                fallback = Some((0, device));
            }
        }

        fallback.map(|(_, device)| device).ok_or_else(|| {
            windows::core::Error::new(
                DXGI_ERROR_UNSUPPORTED,
                if require_mesh {
                    "no adapter supports Direct3D 12 with Shader Model 6.6 and mesh shaders"
                } else {
                    "no adapter supports Direct3D 12 with Shader Model 6.6"
                },
            )
        })
    }
}

/// One line for the log: which card, what kind, how much memory it calls its own.
pub fn describe_device(device: &ID3D12Device) -> windows::core::Result<String> {
    // SAFETY: the LUID comes from the device, and the description arrays are
    // fixed-size and NUL-terminated by DXGI.
    unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
        let adapter: IDXGIAdapter1 = factory.EnumAdapterByLuid(device.GetAdapterLuid())?;
        let desc = adapter.GetDesc1()?;
        let end = desc
            .Description
            .iter()
            .position(|ch| *ch == 0)
            .unwrap_or(desc.Description.len());
        let name = String::from_utf16_lossy(&desc.Description[..end]);
        let mut architecture = D3D12_FEATURE_DATA_ARCHITECTURE1::default();
        let kind = if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
            "WARP"
        } else if !has(device, D3D12_FEATURE_ARCHITECTURE1, &mut architecture) {
            "hardware"
        } else if architecture.UMA.as_bool() {
            "integrated / UMA"
        } else {
            "discrete"
        };
        let gib = desc.DedicatedVideoMemory as f64 / (1u64 << 30) as f64;
        Ok(format!("{name} | {kind} | {gib:.1} GiB dedicated"))
    }
}

fn has<T>(device: &ID3D12Device, feature: D3D12_FEATURE, data: &mut T) -> bool {
    // SAFETY: the callers pair each feature identifier with its own structure.
    unsafe {
        device
            .CheckFeatureSupport(feature, std::ptr::from_mut(data).cast(), std::mem::size_of::<T>() as u32)
            .is_ok()
    }
}
