pub mod helpers;
pub mod hyprland_toplevel_export;

use std::fs::File;

use helpers::{create_shm_buffer, save_frame_to_file};
use hyprland_toplevel_export::hyprland_toplevel_export_frame_v1::{
    Event as FrameEvent, HyprlandToplevelExportFrameV1,
};
use hyprland_toplevel_export::hyprland_toplevel_export_manager_v1::{
    Event as ManagerEvent, HyprlandToplevelExportManagerV1,
};
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    globals::GlobalListContents,
    protocol::{wl_buffer, wl_registry, wl_shm, wl_shm_pool},
};

pub struct AppData {
    pub shm: Option<wl_shm::WlShm>,
    pub export_manager: Option<HyprlandToplevelExportManagerV1>,
    pub frame_info: Option<FrameInfo>,
    pub buffer: Option<wl_buffer::WlBuffer>,
    pub shm_file: Option<File>,
    pub frame_captured: bool,
}

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub format: WEnum<wl_shm::Format>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

macro_rules! impl_empty_dispatch {
    ($proxy_ty:ty, $event_ty:ty, $data_ty:ty) => {
        impl Dispatch<$proxy_ty, $data_ty> for AppData {
            fn event(
                _state: &mut Self,
                _proxy: &$proxy_ty,
                _event: $event_ty,
                _data: &$data_ty,
                _conn: &Connection,
                _qhandle: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

impl_empty_dispatch!(
    wl_registry::WlRegistry,
    wl_registry::Event,
    GlobalListContents
);
impl_empty_dispatch!(wl_shm::WlShm, wl_shm::Event, ());
impl_empty_dispatch!(HyprlandToplevelExportManagerV1, ManagerEvent, ());
impl_empty_dispatch!(wl_shm_pool::WlShmPool, wl_shm_pool::Event, ());
impl_empty_dispatch!(wl_buffer::WlBuffer, wl_buffer::Event, ());

impl Dispatch<HyprlandToplevelExportFrameV1, ()> for AppData {
    fn event(
        state: &mut Self,
        frame: &HyprlandToplevelExportFrameV1,
        event: FrameEvent,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            FrameEvent::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                println!(
                    "Buffer info: {}x{}, stride: {}, format: {:?}",
                    width, height, stride, format
                );
                state.frame_info = Some(FrameInfo {
                    format,
                    width,
                    height,
                    stride,
                });
            }

            FrameEvent::BufferDone => {
                if let (Some(shm), Some(info)) = (&state.shm, &state.frame_info) {
                    let size = info.stride * info.height;
                    match create_shm_buffer(
                        shm,
                        qhandle,
                        size,
                        info.width,
                        info.height,
                        info.stride,
                        info.format.clone(),
                    ) {
                        Ok((buffer, shm_file)) => {
                            frame.copy(&buffer, 0);
                            state.buffer = Some(buffer);
                            state.shm_file = Some(shm_file);
                            println!("Copy request sent");
                        }
                        Err(e) => {
                            eprintln!("Failed to create buffer: {}", e);
                            state.frame_captured = true;
                        }
                    }
                }
            }

            FrameEvent::Flags { flags } => {
                println!("Frame flags: {:?}", flags);
            }

            FrameEvent::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let sec = ((tv_sec_hi as u64) << 32) | (tv_sec_lo as u64);
                println!("Frame ready! Timestamp: {}.{:09} seconds", sec, tv_nsec);

                // Save the frame to a file
                if let (Some(shm_file), Some(info)) = (&state.shm_file, &state.frame_info) {
                    match save_frame_to_file(shm_file, info) {
                        Ok(_) => println!("Frame saved successfully"),
                        Err(e) => eprintln!("Failed to save frame: {}", e),
                    }
                }

                state.frame_captured = true;
            }

            FrameEvent::Failed => {
                eprintln!("Frame capture failed!");
                state.frame_captured = true;
            }

            FrameEvent::Damage {
                x,
                y,
                width,
                height,
            } => {
                println!("Damage region: {}x{} at ({}, {})", width, height, x, y);
            }

            FrameEvent::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                println!(
                    "Linux dmabuf available: {}x{}, format: {}",
                    width, height, format
                );
            }
        }
    }
}
