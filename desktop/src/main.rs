//! The desktop app: a window onto the daemon's client.

use std::process::ExitCode;
use std::time::Duration;

use desktop::{Ask, Edge, Framing};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Icon, ResizeDirection, Theme, WindowBuilder};
use wry::WebViewBuilder;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let asked = match desktop::Asked::read(&args) {
        Ok(asked) => asked,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    if asked.help {
        println!(
            "{name}: the desktop app.\n\n\
             usage: desktop [--url ADDRESS] [--no-start] [--system-frame]\n\n\
             Opens the client in a window. The daemon is looked for at --url, then\n\
             {prefix}_URL, then where you last signed in, then {default}. If it is\n\
             on this machine and not running, it is started, and it keeps running\n\
             after the window is closed.\n\n\
             --no-start starts nothing: the window opens onto a daemon that is\n\
             answering at the address already, and where none is it says so and\n\
             ends. For an address that only looks as if it is on this machine, as\n\
             a daemon in WSL does from Windows (--url http://127.0.0.1:7700).\n\n\
             The window draws its own top bar. --system-frame gives it the\n\
             system's title bar instead, for a system on which that works better.",
            name = brand::DISPLAY,
            prefix = brand::ENV_PREFIX,
            default = brand::DEFAULT_URL
        );
        return ExitCode::SUCCESS;
    }
    let env = std::env::var(brand::env_var("URL")).ok();
    let saved = desktop::saved_address(&brand::config_dir());
    let url = desktop::address(asked.url.as_deref(), env.as_deref(), saved.as_deref());

    // With links followed: the app is installed as a copy in a folder of
    // its own, beside the daemon it starts, and run through a link to it.
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| kernel::whole::whole(&exe).unwrap_or(exe))
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf));
    let program = desktop::daemon_program(beside.as_deref());
    let trouble = if asked.no_start {
        // Asked of the address and of nothing else, and before anything
        // of a window is made: where nobody answers, nothing is shown.
        if !desktop::answers(&url) {
            eprintln!("error: {}", desktop::unanswered(&url));
            return ExitCode::FAILURE;
        }
        None
    } else {
        desktop::ensure_running(&url, &program, Duration::from_secs(15)).err()
    };
    // Opened signed in, by a link the daemon's own program asks for.
    let opening = match &trouble {
        None => desktop::signed_in_address(&url, &program),
        Some(_) => url.clone(),
    };
    let framing = Framing::choose(asked.system_frame);

    // The client's messages arrive in the middle of the web view's own
    // work; they are handed to the loop below, which owns the window.
    let event_loop = EventLoopBuilder::<Ask>::with_user_event().build();
    let to_window = event_loop.create_proxy();
    // The window's own icon, where a system gives a window one (Linux and
    // Windows; on macOS it is the application's, which a bundle carries).
    // Without it the window is still a window.
    let icon = Icon::from_rgba(desktop::ICON.to_vec(), desktop::ICON_SIDE, desktop::ICON_SIDE).ok();
    let window_builder = WindowBuilder::new()
        .with_title(brand::DISPLAY)
        .with_window_icon(icon)
        .with_inner_size(LogicalSize::new(1360.0, 860.0))
        .with_min_inner_size(LogicalSize::new(880.0, 560.0))
        // The client draws the top of the window; the system's bar would be
        // a second one above it.
        .with_decorations(framing != Framing::Page);
    #[cfg(target_os = "macos")]
    let window_builder = if framing == Framing::Overlay {
        use tao::platform::macos::WindowBuilderExtMacOS;
        window_builder
            .with_titlebar_transparent(true)
            .with_fullsize_content_view(true)
            .with_title_hidden(true)
    } else {
        window_builder
    };
    let window = match window_builder.build(&event_loop) {
        Ok(window) => window,
        Err(err) => {
            eprintln!("error: could not open a window: {err}");
            return ExitCode::FAILURE;
        }
    };
    let builder = match &trouble {
        None => WebViewBuilder::new().with_url(&opening),
        Some(message) => WebViewBuilder::new().with_html(desktop::trouble_page(message)),
    };
    let builder =
        builder.with_initialization_script(desktop::page_setup(framing)).with_ipc_handler(move |request| {
            if let Some(ask) = Ask::from_message(request.body()) {
                let _ = to_window.send_event(ask);
            }
        });
    // The window shows the daemon it was opened for and nothing else. A
    // page of somewhere else is for the system's browser, which has no
    // session of this daemon's and no say over this window.
    let daemon = url.clone();
    let builder = builder.with_navigation_handler(move |to| match desktop::going(&daemon, &to) {
        desktop::Going::Here => true,
        desktop::Going::Outside => {
            desktop::open_outside(&to);
            false
        }
        desktop::Going::Nowhere => false,
    });
    let daemon = url.clone();
    // The same for a window a page asks to have opened: one of the
    // daemon's own is the web view's to open as it does, anything else is
    // not opened here.
    let builder = builder.with_new_window_req_handler(move |to, _| match desktop::going(&daemon, &to) {
        desktop::Going::Here => wry::NewWindowResponse::Allow,
        desktop::Going::Outside => {
            desktop::open_outside(&to);
            wry::NewWindowResponse::Deny
        }
        desktop::Going::Nowhere => wry::NewWindowResponse::Deny,
    });
    // Light or dark as the system is set, where the web view would get that
    // wrong left to itself.
    if let Some(dark) = desktop::system_dark() {
        window.set_theme(Some(if dark { Theme::Dark } else { Theme::Light }));
    }
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window);
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        match window.default_vbox() {
            Some(container) => builder.build_gtk(container),
            None => builder.build(&window),
        }
    };
    let _webview = match webview {
        Ok(webview) => webview,
        Err(err) => {
            eprintln!("error: this system's web view could not be started: {err}");
            eprintln!("The client is also at {url} in any browser.");
            return ExitCode::FAILURE;
        }
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // Only the window closes. The daemon is the fleet's conductor
            // and carries on.
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } | Event::UserEvent(Ask::Close) => {
                *control_flow = ControlFlow::Exit;
            }
            // The client's bar standing in for the system's. Each of these
            // can be refused by the system (a tiling one will not be
            // dragged), and then nothing happens, which is right.
            Event::UserEvent(Ask::Drag) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(Ask::Resize(edge)) => {
                let _ = window.drag_resize_window(match edge {
                    Edge::North => ResizeDirection::North,
                    Edge::South => ResizeDirection::South,
                    Edge::East => ResizeDirection::East,
                    Edge::West => ResizeDirection::West,
                    Edge::NorthEast => ResizeDirection::NorthEast,
                    Edge::NorthWest => ResizeDirection::NorthWest,
                    Edge::SouthEast => ResizeDirection::SouthEast,
                    Edge::SouthWest => ResizeDirection::SouthWest,
                });
            }
            Event::UserEvent(Ask::Minimize) => window.set_minimized(true),
            Event::UserEvent(Ask::Maximize) => window.set_maximized(!window.is_maximized()),
            _ => {}
        }
    })
}
