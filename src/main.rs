mod connection;
mod db;
mod export;
mod query;
mod ui;

use gpui::*;
use gpui_platform::application;
use std::sync::OnceLock;

actions!(
    querybox,
    [
        Quit,
        CloseTab,
        SwitchToTab1,
        SwitchToTab2,
        SwitchToTab3,
        SwitchToTab4,
        SwitchToTab5,
        SwitchToTab6,
        SwitchToTab7,
        SwitchToTab8,
        SwitchToTab9,
    ]
);

static DB_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn db_runtime() -> &'static tokio::runtime::Runtime {
    DB_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime for DB")
    })
}

fn main() {
    env_logger::init();
    // Initialize the DB runtime before starting the UI so it's ready on first use.
    db_runtime();
    application().run(|cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("cmd-1", SwitchToTab1, None),
            KeyBinding::new("cmd-2", SwitchToTab2, None),
            KeyBinding::new("cmd-3", SwitchToTab3, None),
            KeyBinding::new("cmd-4", SwitchToTab4, None),
            KeyBinding::new("cmd-5", SwitchToTab5, None),
            KeyBinding::new("cmd-6", SwitchToTab6, None),
            KeyBinding::new("cmd-7", SwitchToTab7, None),
            KeyBinding::new("cmd-8", SwitchToTab8, None),
            KeyBinding::new("cmd-9", SwitchToTab9, None),
        ]);
        cx.set_menus([Menu::new("QueryBox").items([MenuItem::action("Quit", Quit)])]);

        ui::text_field::register_text_field_actions(cx);
        ui::sql_editor::register_sql_editor_actions(cx);
        ui::table_view::register_table_view_actions(cx);
        ui::editor_view::register_editor_view_actions(cx);
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(ui::app_view::AppView::new),
        )
        .unwrap();
        cx.activate(true);
    });
}
