pub mod toplevel_streamer_lib;

use anyhow::{Context, Result};
use wayland_client::{Connection, globals::registry_queue_init, protocol::wl_shm};

use toplevel_streamer_lib::AppData;
use toplevel_streamer_lib::hyprland_toplevel_export::hyprland_toplevel_export_manager_v1::HyprlandToplevelExportManagerV1;

fn main() -> Result<()> {
    // Connect to Wayland
    let conn = Connection::connect_to_env()
        .context("Failed to connect to Wayland. Is WAYLAND_DISPLAY set?")?;

    let (globals, mut event_queue) =
        registry_queue_init::<AppData>(&conn).context("Failed to initialize registry")?;

    let qhandle = event_queue.handle();

    let shm: wl_shm::WlShm = globals
        .bind(&qhandle, 1..=1, ())
        .context("wl_shm not available")?;

    let export_manager: HyprlandToplevelExportManagerV1 = globals
        .bind(&qhandle, 1..=2, ())
        .context("hyprland_toplevel_export_manager_v1 not available - are you running Hyprland?")?;

    let mut state = AppData {
        shm: Some(shm),
        export_manager: Some(export_manager.clone()),
        frame_info: None,
        buffer: None,
        shm_file: None,
        frame_captured: false,
    };

    // Parse window handle from command line
    let window_handle: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            eprintln!("Error: No window handle provided");
            eprintln!(
                "Usage: {} <window_handle_decimal>",
                std::env::args()
                    .next()
                    .unwrap_or_else(|| "program".to_string())
            );
            eprintln!("\nRun ./get_window_handle.sh to get available window handles");
            std::process::exit(1);
        });

    println!("Hyprland Toplevel Export Streamer");
    println!("==================================");
    println!(
        "Target window handle: 0x{:x} ({})",
        window_handle, window_handle
    );

    // Capture the toplevel
    // Note: The protocol expects uint (u32), but window handles are 64-bit addresses
    // We cast to u32 - typically only the lower 32 bits matter for the protocol
    let _frame = export_manager.capture_toplevel(1, window_handle as u32, &qhandle, ());

    // Event loop
    println!("Waiting for frame events...");
    while !state.frame_captured {
        event_queue
            .blocking_dispatch(&mut state)
            .context("Error during event dispatch")?;
    }

    println!("\nCapture complete!");

    Ok(())
}
