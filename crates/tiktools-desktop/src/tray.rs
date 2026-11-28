#[cfg(not(target_os = "linux"))]
use crate::event::CoalescedSignals;
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
#[cfg(not(target_os = "linux"))]
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use winit::event_loop::EventLoopProxy;

#[cfg(not(target_os = "linux"))]
use crate::event::DesktopCommand;
use crate::{
    event::{DesktopEvent, TRAY_MENU_DEVTOOLS_ID, TRAY_MENU_QUIT_ID, TRAY_MENU_SHOW_ID},
    icon,
};

pub struct TrayController {
    _icon: TrayIcon,
}

impl Drop for TrayController {
    fn drop(&mut self) {
        // tray-icon dispatches this callback globally. Clear it whenever the
        // one active tray is destroyed so a later recreation cannot leave a
        // callback holding an old event-loop proxy alive. Menu events use the
        // process-wide dispatcher installed at startup instead.
        #[cfg(not(target_os = "linux"))]
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}

impl TrayController {
    pub fn create(proxy: EventLoopProxy<DesktopEvent>) -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let show = MenuItem::with_id(TRAY_MENU_SHOW_ID, "Show TikTools", true, None);
        let quit = MenuItem::with_id(TRAY_MENU_QUIT_ID, "Quit", true, None);
        menu.append(&show)?;
        // The inspector is only offered when it can actually open (debug
        // builds or the `devtools` cargo feature); release menus stay clean.
        if cfg!(debug_assertions) || cfg!(feature = "devtools") {
            let devtools =
                MenuItem::with_id(TRAY_MENU_DEVTOOLS_ID, "Open Developer Tools", true, None);
            menu.append(&devtools)?;
        }
        menu.append(&quit)?;

        let icon = TrayIconBuilder::new()
            .with_icon(Icon::from_rgba(icon::rgba(), icon::SIZE, icon::SIZE)?)
            .with_tooltip("TikTools")
            .with_menu(Box::new(menu))
            .build()?;

        // The AppIndicator backend used by tray-icon on Linux does not emit
        // TrayIconEvent click notifications. Keep the native click handler on
        // platforms that support it and use the menu event for Linux.
        #[cfg(not(target_os = "linux"))]
        {
            let tray_id = icon.id().clone();
            TrayIconEvent::set_event_handler(Some(move |event| {
                if let TrayIconEvent::Click {
                    id,
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if id == tray_id {
                        send_command(&proxy, DesktopCommand::ShowWindow, "tray click");
                    }
                }
            }));
        }
        #[cfg(target_os = "linux")]
        let _ = proxy;

        // Menu activation is process-global in muda and handled by the
        // dispatcher installed in `event::install_menu_event_handler`.
        Ok(Self { _icon: icon })
    }
}

#[cfg(not(target_os = "linux"))]
fn send_command(
    proxy: &EventLoopProxy<DesktopEvent>,
    command: DesktopCommand,
    source: &'static str,
) {
    if let Err(error) = proxy.send_coalesced(DesktopEvent::Command(command)) {
        tracing::debug!(%error, source, "could not forward system tray command");
    }
}
