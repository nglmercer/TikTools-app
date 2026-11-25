//! Probe: native-Wayland webview via wry's `build_gtk` in a plain GTK window.
//! Run: GDK_BACKEND=wayland cargo run -p tiktools-desktop --example gtk_wayland_probe
use gtk::prelude::*;
use wry::{
    dpi::{LogicalPosition, LogicalSize},
    Rect, WebViewBuilder, WebViewBuilderExtUnix,
};

fn main() {
    gtk::init().expect("gtk init");
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("gtk-wayland-probe");
    window.set_default_size(900, 700);
    let fixed = gtk::Fixed::new();
    window.add(&fixed);
    window.show_all();

    let _webview = WebViewBuilder::new()
        .with_url("https://duckduckgo.com")
        .with_devtools(true)
        .with_bounds(Rect {
            position: LogicalPosition::new(0, 0).into(),
            size: LogicalSize::new(880, 660).into(),
        })
        .build_gtk(&fixed)
        .expect("build_gtk");

    window.connect_destroy(|_| gtk::main_quit());
    gtk::main();
}
