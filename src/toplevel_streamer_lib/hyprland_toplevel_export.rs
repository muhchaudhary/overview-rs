use wayland_client;
use wayland_client::protocol::*;

pub mod __interfaces {
    use wayland_client::protocol::__interfaces::*;
    pub mod wlr_foreign_toplevel {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!(
            "./protocols/wlr-foreign-toplevel-management-unstable-v1.xml"
        );
    }
    use wlr_foreign_toplevel::*;
    wayland_scanner::generate_interfaces!("./protocols/hyprland-toplevel-export-v1.xml");
}

use self::__interfaces::wlr_foreign_toplevel::*;
use self::__interfaces::*;
wayland_scanner::generate_client_code!(
    "./protocols/wlr-foreign-toplevel-management-unstable-v1.xml"
);
wayland_scanner::generate_client_code!("./protocols/hyprland-toplevel-export-v1.xml");
