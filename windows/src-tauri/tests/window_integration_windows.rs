#[cfg(windows)]
mod windows_integration {
    use std::mem::{size_of, zeroed};

    use ai_token_meter_windows::platform::windows::window_controller::{
        Edge, PhysicalSize, meter_shape_points,
    };
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePolygonRgn, DeleteObject, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MonitorFromPoint, PtInRegion, WINDING,
    };

    #[test]
    fn s_curve_builds_a_valid_win32_region_on_both_edges() {
        for edge in [Edge::Left, Edge::Right] {
            let points = meter_shape_points(PhysicalSize::new(116, 450), edge)
                .into_iter()
                .map(|point| POINT {
                    x: point.x,
                    y: point.y,
                })
                .collect::<Vec<_>>();
            let region = unsafe { CreatePolygonRgn(points.as_ptr(), points.len() as i32, WINDING) };
            assert!(!region.is_null(), "Win32 accepted the S-curve polygon");
            assert_ne!(unsafe { PtInRegion(region, 58, 225) }, 0);
            assert_eq!(unsafe { PtInRegion(region, 58, -1) }, 0);
            unsafe { DeleteObject(region) };
        }
    }

    #[test]
    fn windows_exposes_a_nonempty_primary_monitor_and_work_area() {
        let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST) };
        assert!(!monitor.is_null());
        let mut info: MONITORINFO = unsafe { zeroed() };
        info.cbSize = size_of::<MONITORINFO>() as u32;
        assert_ne!(unsafe { GetMonitorInfoW(monitor, &mut info) }, 0);
        assert!(info.rcMonitor.right > info.rcMonitor.left);
        assert!(info.rcMonitor.bottom > info.rcMonitor.top);
        assert!(info.rcWork.right > info.rcWork.left);
        assert!(info.rcWork.bottom > info.rcWork.top);
    }
}
