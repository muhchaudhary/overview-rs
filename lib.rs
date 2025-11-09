use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::{
    define_stub_info_gatherer, derive::gen_stub_pyclass, derive::gen_stub_pymethods,
};
use std::sync::Arc;
mod toplevel_streamer_lib;

use toplevel_streamer_lib::AppData;
use toplevel_streamer_lib::hyprland_toplevel_export::hyprland_toplevel_export_manager_v1::HyprlandToplevelExportManagerV1;
use wayland_client::{Connection, globals::registry_queue_init, protocol::wl_shm};

/// Python class representing frame information
#[pyclass]
#[derive(Clone)]
#[gen_stub_pyclass]
struct PyFrameInfo {
    #[pyo3(get)]
    width: u32,
    #[pyo3(get)]
    height: u32,
    #[pyo3(get)]
    stride: u32,
    #[pyo3(get)]
    format: String,
}

/// Python class for capturing frames from Hyprland windows
#[pyclass]
#[gen_stub_pyclass]
struct HyprlandFrameCapture {
    // Keep connection and state alive
    _conn: Arc<Connection>,
}

#[pymethods]
#[gen_stub_pymethods]
impl HyprlandFrameCapture {
    /// Create a new HyprlandFrameCapture instance
    #[new]
    fn new() -> PyResult<Self> {
        let conn = Connection::connect_to_env().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to connect to Wayland: {}",
                e
            ))
        })?;

        Ok(Self {
            _conn: Arc::new(conn),
        })
    }

    /// Capture a frame from a window by its handle
    ///
    /// Args:
    ///     window_handle: The window handle as a 64-bit integer (get from hyprctl clients)
    ///     overlay_cursor: Whether to include the cursor in the capture (default: 1 = yes)
    ///
    /// Returns:
    ///     A tuple of (frame_data: bytes, frame_info: PyFrameInfo)
    fn capture_frame(
        &self,
        window_handle: u64,
        overlay_cursor: Option<i32>,
    ) -> PyResult<(Py<PyBytes>, PyFrameInfo)> {
        let cursor = overlay_cursor.unwrap_or(1);
        self._capture_frame_internal(window_handle, cursor)
    }
}

impl HyprlandFrameCapture {
    fn _capture_frame_internal(
        &self,
        window_handle: u64,
        overlay_cursor: i32,
    ) -> PyResult<(Py<PyBytes>, PyFrameInfo)> {
        // Connect to Wayland (create fresh connection for each capture)
        let conn = Connection::connect_to_env().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to connect to Wayland: {}",
                e
            ))
        })?;

        let (globals, mut event_queue) = registry_queue_init::<AppData>(&conn).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to initialize registry: {}",
                e
            ))
        })?;

        let qhandle = event_queue.handle();

        let shm: wl_shm::WlShm = globals.bind(&qhandle, 1..=1, ()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "wl_shm not available: {}",
                e
            ))
        })?;

        let export_manager: HyprlandToplevelExportManagerV1 =
            globals.bind(&qhandle, 1..=2, ()).map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "hyprland_toplevel_export_manager_v1 not available - are you running Hyprland?",
                )
            })?;

        let mut state = AppData {
            shm: Some(shm),
            // export_manager: Some(export_manager.clone()),
            frame_info: None,
            buffer: None,
            shm_file: None,
            frame_captured: false,
            captured_buffer: None,
        };

        // Start capture
        let _frame =
            export_manager.capture_toplevel(overlay_cursor, window_handle as u32, &qhandle, ());

        // Event loop - wait for frame to be captured
        while !state.frame_captured {
            event_queue.blocking_dispatch(&mut state).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Error during event dispatch: {}",
                    e
                ))
            })?;
        }

        // Extract the captured data
        let buffer = state.captured_buffer.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Frame capture failed - no buffer data",
            )
        })?;

        let frame_info = state.frame_info.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Frame capture failed - no frame info",
            )
        })?;

        // Convert to Python types
        let py_frame_info = PyFrameInfo {
            width: frame_info.width,
            height: frame_info.height,
            stride: frame_info.stride,
            format: format!("{:?}", frame_info.format),
        };

        // Convert buffer to Python bytes
        Python::attach(|py| {
            let py_bytes = PyBytes::new(py, &buffer).into();
            Ok((py_bytes, py_frame_info))
        })
    }
}

/// A Python module for capturing frames from Hyprland windows
#[pymodule]
fn hyprland_overview_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HyprlandFrameCapture>()?;
    m.add_class::<PyFrameInfo>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info); // Define gatherer for stub info
