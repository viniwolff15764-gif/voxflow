use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

fn reveal(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        crate::position_widget(&window, 56.0, 56.0);
        let _ = window.set_always_on_top(true);
        let _ = window.show();
    }
}

pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let show = MenuItemBuilder::with_id("show", "Mostrar VoxFlow")
        .build(app)
        .map_err(|e| e.to_string())?;
    let settings = MenuItemBuilder::with_id("settings", "Configurações")
        .build(app)
        .map_err(|e| e.to_string())?;
    let quit = MenuItemBuilder::with_id("quit", "Sair")
        .build(app)
        .map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &settings, &quit])
        .build()
        .map_err(|e| e.to_string())?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("VoxFlow — Ditado por voz");

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => reveal(app),
            "settings" => {
                reveal(app);
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}
