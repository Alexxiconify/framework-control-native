use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::*;

const EC_MEMMAP_SIZE: usize = 255;
const HEADER_LEN: usize = 8;
const CROSEC_CMD_MAX_REQUEST: usize = 0x100;
const FILE_DEVICE_CROS_EC: u32 = 0x80EC;

const IOCTL_CROSEC_XCMD: u32 = ((FILE_DEVICE_CROS_EC) << 16) + ((0x3) << 14) + ((0x801) << 2) + 0;
const IOCTL_CROSEC_RDMEM: u32 = ((FILE_DEVICE_CROS_EC) << 16) + ((0x1) << 14) + ((0x802) << 2) + 0;

static mut EC_HANDLE: Option<HANDLE> = None;

fn init_ec() -> bool {
    unsafe {
        if EC_HANDLE.is_some() {
            return true;
        }

        let path = w!(r"\\.\GLOBALROOT\Device\CrosEC");
        match CreateFileW(
            path,
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        ) {
            Ok(h) => {
                EC_HANDLE = Some(h);
                true
            }
            Err(_) => false,
        }
    }
}

pub fn read_ec_memory(offset: u16, length: u16) -> Option<Vec<u8>> {
    if !init_ec() {
        return None;
    }

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
        let handle = EC_HANDLE?;
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

    Some(rm.buffer[..(length as usize)].to_vec())
}

pub fn send_ec_command(command: u16, version: u8, data: &[u8]) -> Option<Vec<u8>> {
    if !init_ec() {
        return None;
    }

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

    unsafe {
        let handle = EC_HANDLE?;
        let mut returned: u32 = 0;
        let _ = DeviceIoControl(
            handle,
            IOCTL_CROSEC_XCMD,
            Some(&mut cmd as *mut _ as *mut _),
            (std::mem::size_of::<EcCommand>() - HEADER_LEN) as u32,
            Some(&mut cmd as *mut _ as *mut _),
            (std::mem::size_of::<EcCommand>() - HEADER_LEN) as u32,
            Some(&mut returned),
            None,
        );

        if cmd.result != 0 {
            return None;
        }

        let end = returned.min(CROSEC_CMD_MAX_REQUEST as u32) as usize;
        Some(cmd.buffer[..end].to_vec())
    }
}

pub fn set_fan_duty(percent: u32) -> bool {
    let data = [percent as u8];
    send_ec_command(0x13, 0, &data).is_some()
}

pub fn set_fan_auto() -> bool {
    send_ec_command(0x14, 0, &[]).is_some()
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
    send_ec_command(0x30, 0, &data).is_some()
}
