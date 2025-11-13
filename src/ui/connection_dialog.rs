use std::collections::HashMap;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::connection::{
    profile::{ConnectionProfile, DatabaseEngine},
    storage,
};
use super::text_field::TextField;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum ConnStatus {
    Checking,
    Ok,
    Failed,
}

#[derive(Debug, Clone)]
enum TestState {
    Idle,
    Testing,
    Ok,
    Failed(String),
}

pub enum ConnectionDialogEvent {
    Connect { profile: ConnectionProfile, password: String },
}

impl EventEmitter<ConnectionDialogEvent> for ConnectionDialog {}

// ── Struct ────────────────────────────────────────────────────────────────────

pub struct ConnectionDialog {
    pub visible: bool,
    focus_handle: FocusHandle,

    // Saved connections panel
    saved_profiles: Vec<ConnectionProfile>,
    conn_statuses: HashMap<String, ConnStatus>,

    // Form
    form_profile_id: Option<String>,
    engine: DatabaseEngine,
    name_field: Entity<TextField>,
    host_field: Entity<TextField>,
    port_field: Entity<TextField>,
    user_field: Entity<TextField>,
    password_field: Entity<TextField>,
    database_field: Entity<TextField>,

    // Test result
    test_state: TestState,
}

impl ConnectionDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let saved_profiles = storage::load_profiles().unwrap_or_default();
        let conn_statuses = saved_profiles.iter()
            .map(|p| (p.id.clone(), ConnStatus::Checking))
            .collect();

        let name_field = cx.new(|cx| {
            let mut f = TextField::new(cx, "Connection name");
            f.set_content("New Connection", cx);
            f
        });
        let host_field = cx.new(|cx| {
            let mut f = TextField::new(cx, "127.0.0.1");
            f.set_content("127.0.0.1", cx);
            f
        });
        let port_field = cx.new(|cx| {
            let mut f = TextField::new(cx, "3306");
            f.set_content("3306", cx);
            f
        });
        let user_field = cx.new(|cx| {
            let mut f = TextField::new(cx, "root");
            f.set_content("root", cx);
            f
        });
        let password_field = cx.new(|cx| {
            let mut f = TextField::new(cx, "Password");
            f.masked = true;
            f
        });
        let database_field = cx.new(|cx| TextField::new(cx, "Database (optional)"));

        // Kick off status checks after entity is ready
        cx.spawn(async move |this: WeakEntity<ConnectionDialog>, cx: &mut AsyncApp| {
            this.update(cx, |d, cx| d.start_status_checks(cx)).ok();
        })
        .detach();

        Self {
            visible: true,
            focus_handle: cx.focus_handle(),
            saved_profiles,
            conn_statuses,
            form_profile_id: None,
            engine: DatabaseEngine::MySql,
            name_field,
            host_field,
            port_field,
            user_field,
            password_field,
            database_field,
            test_state: TestState::Idle,
        }
    }

    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.saved_profiles = storage::load_profiles().unwrap_or_default();
        for p in &self.saved_profiles {
            self.conn_statuses.insert(p.id.clone(), ConnStatus::Checking);
        }
        self.start_status_checks(cx);
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        cx.notify();
    }

    pub fn to_profile(&self, cx: &App) -> ConnectionProfile {
        let database = self.database_field.read(cx).content.to_string();
        ConnectionProfile {
            id: self.form_profile_id.clone().unwrap_or_else(uuid_v4),
            name: self.name_field.read(cx).content.to_string(),
            engine: self.engine,
            host: self.host_field.read(cx).content.to_string(),
            port: self.port_field.read(cx).content.parse().unwrap_or(3306),
            user: self.user_field.read(cx).content.to_string(),
            default_database: if database.is_empty() { None } else { Some(database) },
            file_path: None,
        }
    }

    pub fn password(&self, cx: &App) -> String {
        self.password_field.read(cx).content.to_string()
    }

    // ── Private actions ───────────────────────────────────────────────────────

    fn reset_form(&mut self, cx: &mut Context<Self>) {
        self.form_profile_id = None;
        self.engine = DatabaseEngine::MySql;
        self.name_field.update(cx, |f, cx| f.set_content("New Connection", cx));
        self.host_field.update(cx, |f, cx| f.set_content("127.0.0.1", cx));
        self.port_field.update(cx, |f, cx| f.set_content("3306", cx));
        self.user_field.update(cx, |f, cx| f.set_content("root", cx));
        self.password_field.update(cx, |f, cx| f.set_content("", cx));
        self.database_field.update(cx, |f, cx| f.set_content("", cx));
        self.test_state = TestState::Idle;
        cx.notify();
    }

    fn load_profile_into_form(&mut self, profile: &ConnectionProfile, cx: &mut Context<Self>) {
        self.form_profile_id = Some(profile.id.clone());
        self.engine = profile.engine;
        self.name_field.update(cx, |f, cx| f.set_content(&profile.name, cx));
        self.host_field.update(cx, |f, cx| f.set_content(&profile.host, cx));
        self.port_field.update(cx, |f, cx| f.set_content(&profile.port.to_string(), cx));
        self.user_field.update(cx, |f, cx| f.set_content(&profile.user, cx));
        let pw = storage::get_password(profile).ok().flatten().unwrap_or_default();
        self.password_field.update(cx, |f, cx| f.set_content(&pw, cx));
        let db = profile.default_database.clone().unwrap_or_default();
        self.database_field.update(cx, |f, cx| f.set_content(&db, cx));
        self.test_state = TestState::Idle;
        cx.notify();
    }

    fn save_profile(&mut self, cx: &mut Context<Self>) {
        let profile = self.to_profile(cx);
        let password = self.password(cx);

        let mut profiles = storage::load_profiles().unwrap_or_default();
        if let Some(pos) = profiles.iter().position(|p| p.id == profile.id) {
            profiles[pos] = profile.clone();
        } else {
            profiles.push(profile.clone());
        }
        storage::save_profiles(&profiles).ok();
        storage::store_password(&profile, &password).ok();

        self.saved_profiles = profiles;
        for p in &self.saved_profiles {
            self.conn_statuses.entry(p.id.clone()).or_insert(ConnStatus::Checking);
        }
        // Mark the just-saved profile as checking and re-test it
        self.conn_statuses.insert(profile.id.clone(), ConnStatus::Checking);
        self.start_status_check_for(profile.id.clone(), profile.clone(), password, cx);

        self.reset_form(cx);
    }

    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let profile = self.to_profile(cx);
        let password = self.password(cx);
        self.test_state = TestState::Testing;
        cx.notify();

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        crate::db_runtime().spawn(async move {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                crate::connection::test_connect(&profile, &password),
            )
            .await;
            let result = match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err("Connection timed out".into()),
            };
            tx.send(result).ok();
        });

        cx.spawn(async move |this: WeakEntity<ConnectionDialog>, cx: &mut AsyncApp| {
            if let Ok(result) = rx.await {
                this.update(cx, |d, cx| {
                    d.test_state = match result {
                        Ok(()) => TestState::Ok,
                        Err(e) => TestState::Failed(e),
                    };
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn connect_saved(&mut self, profile: ConnectionProfile, cx: &mut Context<Self>) {
        let password = storage::get_password(&profile).ok().flatten().unwrap_or_default();
        cx.emit(ConnectionDialogEvent::Connect { profile, password });
    }

    fn delete_profile(&mut self, profile_id: String, cx: &mut Context<Self>) {
        let mut profiles = storage::load_profiles().unwrap_or_default();
        profiles.retain(|p| p.id != profile_id);
        storage::save_profiles(&profiles).ok();
        self.saved_profiles = profiles;
        self.conn_statuses.remove(&profile_id);
        if self.form_profile_id.as_deref() == Some(profile_id.as_str()) {
            self.reset_form(cx);
        } else {
            cx.notify();
        }
    }

    fn start_status_checks(&mut self, cx: &mut Context<Self>) {
        let profiles: Vec<ConnectionProfile> = self.saved_profiles.clone();
        for profile in profiles {
            let pw = storage::get_password(&profile).ok().flatten().unwrap_or_default();
            self.start_status_check_for(profile.id.clone(), profile, pw, cx);
        }
    }

    fn start_status_check_for(
        &mut self,
        profile_id: String,
        profile: ConnectionProfile,
        password: String,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        crate::db_runtime().spawn(async move {
            let ok = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                crate::connection::test_connect(&profile, &password),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);
            tx.send(ok).ok();
        });

        cx.spawn(async move |this: WeakEntity<ConnectionDialog>, cx: &mut AsyncApp| {
            if let Ok(ok) = rx.await {
                this.update(cx, |d, cx| {
                    d.conn_statuses.insert(
                        profile_id,
                        if ok { ConnStatus::Ok } else { ConnStatus::Failed },
                    );
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}

impl Focusable for ConnectionDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        div()
            .absolute()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(rgba(0x00000088))
            .child(
                div()
                    .w(px(680.))
                    .min_h(px(480.))
                    .bg(rgb(0x1e1e2e))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(0x45475a))
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .child(self.render_saved_panel(cx))
                    .child(self.render_form_panel(cx)),
            )
            .into_any_element()
    }
}

impl ConnectionDialog {
    fn render_saved_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panel = div()
            .w(px(210.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(0x181825))
            .border_r_1()
            .border_color(rgb(0x313244));

        // Header
        panel = panel.child(
            div()
                .px(px(14.))
                .py(px(12.))
                .border_b_1()
                .border_color(rgb(0x313244))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x6c7086))
                .child("SAVED CONNECTIONS"),
        );

        // Connection rows
        let mut list = div().flex().flex_col().flex_1().overflow_hidden();
        for profile in &self.saved_profiles {
            let status = self.conn_statuses.get(&profile.id).cloned().unwrap_or(ConnStatus::Checking);
            let dot_color = match status {
                ConnStatus::Checking => rgb(0x585b70),
                ConnStatus::Ok => rgb(0xa6e3a1),
                ConnStatus::Failed => rgb(0xf38ba8),
            };
            let name = profile.name.clone();
            let profile_for_load = profile.clone();
            let profile_for_connect = profile.clone();
            let profile_id_for_delete = profile.id.clone();

            list = list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(10.))
                    .py(px(6.))
                    .gap_2()
                    .hover(|d| d.bg(rgb(0x1e1e2e)))
                    // Status dot
                    .child(
                        div()
                            .w(px(8.))
                            .h(px(8.))
                            .rounded_full()
                            .flex_shrink_0()
                            .bg(dot_color),
                    )
                    // Name — click to load into form
                    .child(
                        div()
                            .id(ElementId::Name(format!("load-{}", profile.id).into()))
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcdd6f4))
                            .overflow_hidden()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.load_profile_into_form(&profile_for_load, cx);
                            }))
                            .child(name),
                    )
                    // Quick-connect arrow
                    .child(
                        div()
                            .id(ElementId::Name(format!("connect-{}", profile_for_connect.id).into()))
                            .text_size(px(13.))
                            .text_color(rgb(0x6c7086))
                            .cursor_pointer()
                            .hover(|d| d.text_color(rgb(0x89b4fa)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.connect_saved(profile_for_connect.clone(), cx);
                            }))
                            .child("→"),
                    )
                    // Delete button
                    .child(
                        div()
                            .id(ElementId::Name(format!("delete-{}", profile_id_for_delete).into()))
                            .text_size(px(13.))
                            .text_color(rgb(0x45475a))
                            .cursor_pointer()
                            .hover(|d| d.text_color(rgb(0xf38ba8)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_profile(profile_id_for_delete.clone(), cx);
                            }))
                            .child("×"),
                    ),
            );
        }

        panel = panel.child(list);

        // New connection button
        panel = panel.child(
            div()
                .id("new-conn-btn")
                .flex()
                .items_center()
                .gap_2()
                .px(px(10.))
                .py(px(10.))
                .border_t_1()
                .border_color(rgb(0x313244))
                .text_size(px(12.))
                .text_color(rgb(0x89b4fa))
                .cursor_pointer()
                .hover(|d| d.bg(rgb(0x1e1e2e)))
                .on_click(cx.listener(|this, _, _, cx| this.reset_form(cx)))
                .child("+ New Connection"),
        );

        panel
    }

    fn render_form_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .p(px(20.))
            .gap_3()
            // Engine selector
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_engine_btn("MySQL", DatabaseEngine::MySql, cx))
                    .child(self.render_engine_btn("PostgreSQL", DatabaseEngine::PostgreSql, cx))
                    .child(self.render_engine_btn("SQLite", DatabaseEngine::Sqlite, cx)),
            )
            // Name
            .child(self.render_labeled("Name", self.name_field.clone()))
            // Host + Port
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(self.render_labeled("Host", self.host_field.clone())))
                    .child(div().w(px(90.)).child(self.render_labeled("Port", self.port_field.clone()))),
            )
            // User + Password
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().flex_1().child(self.render_labeled("User", self.user_field.clone())))
                    .child(div().flex_1().child(self.render_labeled("Password", self.password_field.clone()))),
            )
            // Database
            .child(self.render_labeled("Database (optional)", self.database_field.clone()))
            // Test result
            .child(self.render_test_state())
            .child(div().flex_1())
            // Action buttons
            .child(self.render_buttons(cx))
    }

    fn render_engine_btn(
        &self,
        label: &'static str,
        e: DatabaseEngine,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.engine == e;
        div()
            .id(ElementId::Name(label.into()))
            .px(px(12.))
            .py(px(5.))
            .text_size(px(12.))
            .rounded(px(4.))
            .cursor_pointer()
            .bg(if active { rgb(0x89b4fa) } else { rgb(0x313244) })
            .text_color(if active { rgb(0x1e1e2e) } else { rgb(0xa6adc8) })
            .when(active, |d| d.font_weight(FontWeight::SEMIBOLD))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.engine = e;
                let port = match e {
                    DatabaseEngine::MySql => "3306",
                    DatabaseEngine::PostgreSql => "5432",
                    DatabaseEngine::Sqlite => "",
                };
                this.port_field.update(cx, |f, cx| f.set_content(port, cx));
                this.test_state = TestState::Idle;
                cx.notify();
            }))
            .child(label)
    }

    fn render_labeled(&self, label: &str, field: Entity<TextField>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .child(label.to_string()),
            )
            .child(field)
    }

    fn render_test_state(&self) -> impl IntoElement {
        match &self.test_state {
            TestState::Idle => div().into_any_element(),
            TestState::Testing => div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .rounded_full()
                        .bg(rgb(0x585b70)),
                )
                .child(div().text_size(px(12.)).text_color(rgb(0x6c7086)).child("Testing…"))
                .into_any_element(),
            TestState::Ok => div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .rounded_full()
                        .bg(rgb(0xa6e3a1)),
                )
                .child(div().text_size(px(12.)).text_color(rgb(0xa6e3a1)).child("Connection successful"))
                .into_any_element(),
            TestState::Failed(e) => div()
                .w_full()
                .overflow_hidden()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .flex_shrink_0()
                        .rounded_full()
                        .bg(rgb(0xf38ba8)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(11.))
                        .text_color(rgb(0xf38ba8))
                        .overflow_hidden()
                        .child(e.clone()),
                )
                .into_any_element(),
        }
    }

    fn render_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_testing = matches!(self.test_state, TestState::Testing);
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("dlg-cancel")
                    .px(px(14.))
                    .py(px(6.))
                    .bg(rgb(0x313244))
                    .text_color(rgb(0xa6adc8))
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.hide(cx)))
                    .child("Cancel"),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("dlg-test")
                    .px(px(14.))
                    .py(px(6.))
                    .bg(rgb(0x313244))
                    .text_color(if is_testing { rgb(0x6c7086) } else { rgb(0xa6adc8) })
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(!is_testing, |d| {
                        d.on_click(cx.listener(|this, _, _, cx| this.test_connection(cx)))
                    })
                    .child(if is_testing { "Testing…" } else { "Test" }),
            )
            .child(
                div()
                    .id("dlg-save")
                    .px(px(14.))
                    .py(px(6.))
                    .bg(rgb(0x313244))
                    .text_color(rgb(0xa6adc8))
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.save_profile(cx)))
                    .child("Save"),
            )
            .child(
                div()
                    .id("dlg-connect")
                    .px(px(14.))
                    .py(px(6.))
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let profile = this.to_profile(cx);
                        let password = this.password(cx);
                        cx.emit(ConnectionDialogEvent::Connect { profile, password });
                    }))
                    .child("Connect"),
            )
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", t)
}
