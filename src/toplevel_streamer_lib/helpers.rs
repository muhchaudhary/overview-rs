use super::AppData;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd};
use wayland_client::protocol::{wl_buffer, wl_shm};
use wayland_client::{QueueHandle, WEnum};

pub fn create_shm_buffer(
    shm: &wl_shm::WlShm,
    qhandle: &QueueHandle<AppData>,
    size: u32,
    width: u32,
    height: u32,
    stride: u32,
    format: WEnum<wl_shm::Format>,
) -> Result<(wl_buffer::WlBuffer, File)> {
    // Create anonymous file
    let fd = create_shm_fd(size as usize)?;

    // Create pool
    let pool = shm.create_pool(fd.as_fd(), size as i32, qhandle, ());

    // Create buffer from pool
    // Convert WEnum to concrete type - use the value if known, or default
    let fmt = match format {
        WEnum::Value(f) => f,
        WEnum::Unknown(_) => wl_shm::Format::Argb8888, // Default fallback
    };

    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        fmt,
        qhandle,
        (),
    );

    pool.destroy();

    Ok((buffer, fd))
}

fn create_shm_fd(size: usize) -> Result<std::fs::File> {
    use std::os::unix::io::FromRawFd;

    let name = std::ffi::CString::new("toplevel-export").unwrap();
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };

    if fd < 0 {
        anyhow::bail!("Failed to create memfd");
    }

    unsafe {
        if libc::ftruncate(fd, size as i64) < 0 {
            libc::close(fd);
            anyhow::bail!("Failed to truncate memfd");
        }
    }

    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

pub fn save_frame_to_file(shm_file: &File, info: &super::FrameInfo) -> Result<()> {
    let size = (info.stride * info.height) as usize;

    // Read from shared memory
    let mut buffer = vec![0u8; size];
    unsafe {
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ,
            libc::MAP_SHARED,
            shm_file.as_raw_fd(),
            0,
        );

        if ptr == libc::MAP_FAILED {
            anyhow::bail!("Failed to mmap buffer");
        }

        std::ptr::copy_nonoverlapping(ptr as *const u8, buffer.as_mut_ptr(), size);
        libc::munmap(ptr, size);
    }

    // Save as raw BGRA data
    let filename = "captured_frame.bgra";
    let mut file = File::create(filename)?;
    file.write_all(&buffer)?;

    println!("Frame saved to {}", filename);
    println!("To view, convert with ImageMagick:");
    println!(
        "  convert -size {}x{} -depth 8 bgra:{} output.png",
        info.width, info.height, filename
    );

    Ok(())
}
