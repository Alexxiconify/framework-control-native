use std::sync::OnceLock;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

// Flag to avoid repeated "opened" logs
static EC_OPEN_LOGGED: OnceLock<bool> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum EcError {
    AccessDenied,
    DriverMissing,
    IoError(String),
}

// Open EC device fresh each time - no caching to avoid permission and thread-safety issues
fn get_ec_handle() -> Result<HANDLE, EcError> {
    // Try multiple known CrosEC / crosecbus device paths
    let paths = [
        w!(r"\\.\GLOBALROOT\Device\CrosEC"),
        w!(r"\\.\CrosEC"),
        w!(r"\\.\GLOBALROOT\Device\CrosECDevice"),
        w!(r"\\.\crosecbus"),
        w!(r"\\.\GLOBALROOT\Device\crosecbus"),
        w!(r"\\.\GLOBALROOT\Device\CrosEcBus"),
        w!(r"\\.\crossecbus"),
        w!(r"\\.\GLOBALROOT\Device\crossecbus"),
        w!(r"\\.\GLOBALROOT\Device\CrosSecBus"),
    ];

    for p in paths.iter() {
        let res = unsafe {
            CreateFileW(
                *p,
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };

        match res {
            Ok(h) => {
                if EC_OPEN_LOGGED.get().is_none() {
                    let _ = EC_OPEN_LOGGED.set(true);
                    println!("✅ EC device opened");
                }
                return Ok(h);
            }
            Err(e) => {
                // Check specifically for Access Denied
                if e.code() == ERROR_ACCESS_DENIED.into() {
                    return Err(EcError::AccessDenied);
                }
            }
        }
    }

    // If we get here, we either couldn't find the device or had other errors
    // But if we saw at least one AccessDenied, we should probably report that?
    // For now, if we can't open any, assume driver missing or general failure
    // unless we want to be more specific.
    // Let's return DriverMissing if we simply couldn't find it.
    println!("❌ EC device open failed (all known paths). Ensure the Framework EC driver (crosecbus/crossecbus) is installed.");
    Err(EcError::DriverMissing)
}

fn close_ec_handle(handle: HANDLE) {
    unsafe {
        let _ = CloseHandle(handle);
    }
}

const EC_MEMMAP_SIZE: usize = 255;
const HEADER_LEN: usize = 8;
const CROSEC_CMD_MAX_REQUEST: usize = 0x100;
const FILE_DEVICE_CROS_EC: u32 = 0x80EC;

const IOCTL_CROSEC_XCMD: u32 = ((FILE_DEVICE_CROS_EC) << 16) + ((0x3) << 14) + ((0x801) << 2) + 0;
const IOCTL_CROSEC_RDMEM: u32 = ((FILE_DEVICE_CROS_EC) << 16) + ((0x1) << 14) + ((0x802) << 2) + 0;

pub fn read_ec_memory(offset: u16, length: u16) -> Option<Vec<u8>> {
    let handle = get_ec_handle().ok()?;

    #[repr(C)]
    struct ReadMem {
        offset: u32,
        bytes: u32,
        buffer: [u8; EC_MEMMAP_SIZE],
    }

    let mut rm = ReadMem {
        offset: offset as u32,
        bytes: length as u32,
        buffer: [0u8; EC_MEMMAP_SIZE],
    };

    unsafe {
        let _ = DeviceIoControl(
            handle,
            IOCTL_CROSEC_RDMEM,
            Some(&mut rm as *mut _ as *mut _),
            std::mem::size_of::<ReadMem>() as u32,
            Some(&mut rm as *mut _ as *mut _),
            std::mem::size_of::<ReadMem>() as u32,
            None,
            None,
        );
    }

    close_ec_handle(handle);
    Some(rm.buffer[..(length as usize)].to_vec())
}

pub fn send_ec_command(command: u16, version: u8, data: &[u8]) -> Result<Vec<u8>, EcError> {
    let handle = get_ec_handle()?;

    println!(
        "📤 Sending EC command: 0x{:02X}, version: {}, data len: {}",
        command,
        version,
        data.len()
    );

    #[repr(C)]
    struct EcCommand {
        version: u32,
        command: u32,
        outsize: u32,
        insize: u32,
        result: u32,
        buffer: [u8; CROSEC_CMD_MAX_REQUEST],
    }

    let mut cmd = EcCommand {
        version: version as u32,
        command: command as u32,
        outsize: data.len() as u32,
        insize: (CROSEC_CMD_MAX_REQUEST - HEADER_LEN) as u32,
        result: 0xFF,
        buffer: [0u8; CROSEC_CMD_MAX_REQUEST],
    };

    cmd.buffer[..data.len()].copy_from_slice(data);

    let result = unsafe {
        let mut returned: u32 = 0;
        let io_result = DeviceIoControl(
            handle,
            IOCTL_CROSEC_XCMD,
            Some(&mut cmd as *mut _ as *mut _),
            (std::mem::size_of::<EcCommand>() - HEADER_LEN) as u32,
            Some(&mut cmd as *mut _ as *mut _),
            (std::mem::size_of::<EcCommand>() - HEADER_LEN) as u32,
            Some(&mut returned),
            None,
        );

        if let Err(ref e) = io_result {
            println!("📥 EC IOCTL error: {:?}", e);
            if e.code() == ERROR_ACCESS_DENIED.into() {
                println!("🔒 EC access denied.");
                close_ec_handle(handle);
                return Err(EcError::AccessDenied);
            }
        }

        println!(
            "📥 EC command result: {:?}, returned bytes: {}, cmd.result: {}",
            io_result, returned, cmd.result
        );

        if cmd.result != 0 {
            if cmd.result == 255 {
                // EC_RES_ACCESS_DENIED often maps to this or similar
                println!("❌ EC command blocked by permissions.");
            } else {
                println!("❌ EC command failed with result code: {}", cmd.result);
            }
            close_ec_handle(handle);
            return Err(EcError::IoError(format!("EC result code: {}", cmd.result)));
        }

        let end = returned.min(CROSEC_CMD_MAX_REQUEST as u32) as usize;
        println!("✅ EC command succeeded");
        Ok(cmd.buffer[..end].to_vec())
    };

    close_ec_handle(handle);
    result
}

pub fn set_fan_duty(percent: u32) -> bool {
    let data = [percent as u8];
    send_ec_command(0x13, 0, &data).is_ok()
}

pub fn set_fan_auto() -> bool {
    send_ec_command(0x14, 0, &[]).is_ok()
}

pub fn read_temps() -> Vec<f32> {
    let mut temps = Vec::new();
    if let Some(data) = read_ec_memory(0x00, 0x0F) {
        for &t in &data {
            if t < 0xFC {
                let temp_c = (t as i16 - 73) as f32;
                if temp_c > -50.0 && temp_c < 150.0 {
                    temps.push(temp_c);
                }
            }
        }
    }
    temps
}

pub fn read_fans() -> Vec<f32> {
    let mut fans = Vec::new();
    if let Some(data) = read_ec_memory(0x10, 0x08) {
        for i in 0..4 {
            let offset = i * 2;
            if offset + 1 < data.len() {
                let rpm = u16::from_le_bytes([data[offset], data[offset + 1]]);
                if rpm != 0xFFFF {
                    fans.push(rpm as f32);
                }
            }
        }
    }
    fans
}

pub fn set_charge_limit(max_pct: u8) -> bool {
    let min_pct = if max_pct > 5 { max_pct - 5 } else { 0 };
    let data = [min_pct, max_pct];
    send_ec_command(0x30, 0, &data).is_ok()
}

pub fn set_tdp_watts(tdp: u32) -> bool {
    let data = tdp.to_le_bytes();
    send_ec_command(0x20, 0, &data).is_ok()
}

pub fn set_thermal_limit(limit: u32) -> bool {
    let data = limit.to_le_bytes();
    send_ec_command(0x21, 0, &data).is_ok()
}

pub fn restart_as_admin() {
    unsafe {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let path_str = current_exe.to_str().unwrap_or_default();

        let path_hstring = windows::core::HSTRING::from(path_str);
        let args_hstring = windows::core::HSTRING::from(""); // Pass current args if needed

        let _ = ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(path_hstring.as_ptr()),
            PCWSTR(args_hstring.as_ptr()),
            None,
            SW_SHOW,
        );

        // Exit current process
        std::process::exit(0);
    }
}

pub fn check_connection() -> Result<(), EcError> {
    let handle = get_ec_handle()?;
    close_ec_handle(handle);
    Ok(())
}
