use std::{ffi::OsString, sync::Arc};

use bridge::{
    handle::BackendHandle,
    instance::{InstanceID, InstanceServerSummary, InstanceWorldSummary},
    message::{BridgeDataLoadState, MessageToBackend, QuickPlayLaunch},
    serial::AtomicOptionSerial,
};
use gpui::{prelude::*, *};
use parking_lot::Mutex;
use gpui_component::{
    ActiveTheme as _, Colorize, Disableable, IndexPath, Sizable, Theme, button::{Button, ButtonVariants}, h_flex, list::{List, ListDelegate, ListItem, ListState}, spinner::Spinner, v_flex
};

use crate::{
    entity::{DataEntities, instance::InstanceEntry}, icon::PandoraIcon, interface_config::InterfaceConfig, modals::edit_server::ExistingServer, png_render_cache, root,
};

pub struct InstanceQuickplaySubpage {
    instance: Entity<InstanceEntry>,
    backend_handle: BackendHandle,
    worlds_state: BridgeDataLoadState,
    world_list: Entity<ListState<WorldsListDelegate>>,
    servers_state: BridgeDataLoadState,
    server_list: Entity<ListState<ServersListDelegate>>,
    worlds_serial: AtomicOptionSerial,
    servers_serial: AtomicOptionSerial,
}

impl InstanceQuickplaySubpage {
    pub fn new(
        instance: &Entity<InstanceEntry>,
        data: &DataEntities,
        mut window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let instance_entity = instance.clone();
        let instance = instance.read(cx);
        let instance_id = instance.id;

        let worlds_state = instance.worlds_state.clone();
        let servers_state = instance.servers_state.clone();

        let worlds = instance.worlds.read(cx).clone().map(|l| l.to_vec());
        let worlds_list_delegate = WorldsListDelegate {
            id: instance_id,
            name: instance.name.clone(),
            data: data.clone(),
            loaded: worlds.is_some(),
            worlds: worlds.clone().unwrap_or_default(),
            searched: worlds.unwrap_or_default(),
        };

        let servers = instance.servers.read(cx).clone().map(|l| l.to_vec());
        let servers_list_delegate = ServersListDelegate {
            id: instance_id,
            name: instance.name.clone(),
            data: data.clone(),
            loaded: servers.is_some(),
            servers: servers.clone().unwrap_or_default(),
            searched: servers.unwrap_or_default(),
            search_query: String::new(),
            confirming_delete: Default::default(),
        };

        let worlds = instance.worlds.clone();
        let servers = instance.servers.clone();

        let window2 = &mut window;
        let world_list = cx.new(move |cx| {
            cx.observe(&worlds, |list: &mut ListState<WorldsListDelegate>, worlds, cx| {
                let worlds = worlds.read(cx).clone().map(|l| l.to_vec());
                let delegate = list.delegate_mut();
                delegate.loaded = worlds.is_some();
                delegate.worlds = worlds.clone().unwrap_or_default();
                delegate.searched = worlds.unwrap_or_default();
                cx.notify();
            })
            .detach();

            ListState::new(worlds_list_delegate, window2, cx).selectable(false).searchable(true)
        });

        let server_list = cx.new(move |cx| {
            cx.observe(&servers, |list: &mut ListState<ServersListDelegate>, servers, cx| {
                let servers = servers.read(cx).clone().map(|l| l.to_vec());
                let delegate = list.delegate_mut();
                delegate.set_servers(servers);
                cx.notify();
            })
            .detach();

            ListState::new(servers_list_delegate, window, cx).selectable(false).searchable(true)
        });

        cx.observe(&instance_entity, |_, _, cx| {
            cx.notify();
        })
        .detach();

        Self {
            instance: instance_entity,
            backend_handle: data.backend_handle.clone(),
            worlds_state,
            world_list,
            servers_state,
            server_list,
            worlds_serial: AtomicOptionSerial::default(),
            servers_serial: AtomicOptionSerial::default(),
        }
    }
}

impl Render for InstanceQuickplaySubpage {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme();
        let instance = self.instance.read(cx);
        let playtime = instance.playtime;
        let instance_id = instance.id;

        self.worlds_state.set_observed();
        if self.worlds_state.should_load() {
            self.backend_handle
                .send_with_serial(MessageToBackend::RequestLoadWorlds { id: instance_id }, &self.worlds_serial);
        }

        self.servers_state.set_observed();
        if self.servers_state.should_load() {
            self.backend_handle
                .send_with_serial(MessageToBackend::RequestLoadServers { id: instance_id }, &self.servers_serial);
        }

        let worlds_header = div().mb_1().ml_1().text_lg().child(t::instance::worlds());
        let servers_header = h_flex()
            .mb_1()
            .ml_1()
            .items_center()
            .child(div().flex_1().text_lg().child(t::instance::servers()))
            .child(Button::new("add-server")
                .compact()
                .small()
                .icon(PandoraIcon::Plus)
                .label(t::instance::quickplay::add_server())
                .on_click({
                    let backend_handle = self.backend_handle.clone();
                    move |_, window, cx| {
                        crate::modals::edit_server::open_edit_server(instance_id, None, backend_handle.clone(), window, cx);
                    }
                }));
        let total_playtime = format_playtime(playtime.total_secs);
        let current_session = if playtime.current_session_secs > 0 {
            format_playtime(playtime.current_session_secs)
        } else {
            t::instance::current_session::not_running().into()
        };

        v_flex()
            .p_4()
            .gap_4()
            .size_full()
            .child(
                h_flex()
                    .gap_4()
                    .child(card(
                        t::instance::current_session(),
                        current_session.into(),
                        theme,
                    ))
                    .child(card(
                        t::instance::total_playtime(),
                        total_playtime,
                        theme,
                    )),
            )
            .child(
                h_flex()
                    .size_full()
                    .gap_4()
                    .child(
                        v_flex().size_full().child(worlds_header).child(
                            v_flex()
                                .text_base()
                                .size_full()
                                .border_1()
                                .rounded(theme.radius)
                                .border_color(theme.border)
                                .child(List::new(&self.world_list).search_placeholder(t::common::search())),
                        ),
                    )
                    .child(
                        v_flex().size_full().child(servers_header).child(
                            v_flex()
                                .text_base()
                                .size_full()
                                .border_1()
                                .rounded(theme.radius)
                                .border_color(theme.border)
                                .child(List::new(&self.server_list).search_placeholder(t::common::search())),
                        ),
                    ),
            )
    }
}

fn card(label: impl Into<SharedString>, value: SharedString, theme: &Theme) -> Div {
    v_flex()
        .gap_1()
        .px_3()
        .py_2()
        .min_w_40()
        .border_1()
        .border_color(theme.border)
        .rounded(theme.radius)
        .child(div().text_sm().text_color(theme.muted_foreground).child(label.into()))
        .child(div().text_lg().child(value))
}

fn format_playtime(total_secs: u64) -> SharedString {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{hours}{} {minutes}{}", t::time::h(), t::time::m()).into()
    } else if minutes > 0 {
        format!("{minutes}{} {seconds}{}", t::time::m(), t::time::s()).into()
    } else {
        format!("{seconds}{}", t::time::s()).into()
    }
}

pub struct WorldsListDelegate {
    id: InstanceID,
    name: SharedString,
    data: DataEntities,
    loaded: bool,
    worlds: Vec<InstanceWorldSummary>,
    searched: Vec<InstanceWorldSummary>,
}

impl ListDelegate for WorldsListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.searched.len()
    }

    fn loading(&self, _cx: &App) -> bool {
        !self.loaded
    }

    fn render_loading(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        v_flex()
            .w_full()
            .h_1_2()
            .items_center()
            .justify_center()
            .child(Spinner::new().color(cx.theme().muted_foreground).with_size(px(36.0)))
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let summary = self.searched.get(ix.row)?;

        let icon = if let Some(png_icon) = summary.png_icon.as_ref() {
            png_render_cache::render(png_icon.clone(), cx)
        } else {
            gpui::img(ImageSource::Resource(Resource::Embedded("images/default_world.png".into())))
        };

        let description = v_flex().child(SharedString::from(summary.title.clone())).child(
            div()
                .text_color(Hsla {
                    h: 0.0,
                    s: 0.0,
                    l: 0.5,
                    a: 1.0,
                })
                .child(SharedString::from(summary.subtitle.clone())),
        );

        let id = self.id;
        let name = self.name.clone();
        let data = self.data.clone();
        let target = summary.level_path.file_name().unwrap().to_owned();
        let item = ListItem::new(ix).p_1().child(
            h_flex()
                .gap_1()
                .child(
                    div()
                        .child(Button::new(ix).success().icon(PandoraIcon::Play).on_click(move |_, window, cx| {
                            root::start_instance(
                                id,
                                name.clone(),
                                Some(QuickPlayLaunch::Singleplayer(target.clone())),
                                &data,
                                window,
                                cx,
                            );
                        }))
                        .px_2(),
                )
                .child(icon.size_16().min_w_16().min_h_16())
                .child(description),
        );

        Some(item)
    }

    fn set_selected_index(&mut self, _ix: Option<IndexPath>, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _cx: &mut Context<ListState<Self>>) -> Task<()> {
        self.searched = self.worlds.iter().filter(|w| w.title.contains(query)).cloned().collect();

        Task::ready(())
    }
}

pub struct ServersListDelegate {
    id: InstanceID,
    name: SharedString,
    data: DataEntities,
    loaded: bool,
    servers: Vec<InstanceServerSummary>,
    searched: Vec<InstanceServerSummary>,
    search_query: String,
    confirming_delete: Arc<Mutex<Option<usize>>>,
}

impl ListDelegate for ServersListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.searched.len()
    }

    fn loading(&self, _cx: &App) -> bool {
        !self.loaded
    }

    fn render_loading(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        v_flex()
            .w_full()
            .h_1_2()
            .items_center()
            .justify_center()
            .child(Spinner::new().color(cx.theme().muted_foreground).with_size(px(36.0)))
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let interface_config = InterfaceConfig::get(cx);

        let hide_server_addresses = interface_config.hide_server_addresses;

        let summary = self.searched.get(ix.row)?;
        let can_reorder = self.can_reorder();

        let icon = if let Some(png_icon) = summary.png_icon.as_ref() {
            png_render_cache::render(png_icon.clone(), cx)
        } else {
            gpui::img(ImageSource::Resource(Resource::Embedded("images/default_world.png".into())))
        };

        let ip_text: SharedString = if hide_server_addresses {
            "".into()
        } else {
            format!("({})", summary.ip).into()
        };
        let theme = cx.theme();
        let description = v_flex()
            .min_w_0()
            .w(px(542.0))
            .max_w(px(542.0))
            .gap_2()
            .line_height(rems(1.0))
            .overflow_x_hidden()
            .child(h_flex()
                .gap_2()
                .child(SharedString::from(summary.name.clone()))
                .child(div()
                    .flex_1()
                    .text_color(theme.muted_foreground)
                    .child(ip_text))
                .when_some(summary.status.as_ref(), |this, status| {
                    if let Some(players) = &status.players {
                        this.child(div()
                            .text_color(theme.muted_foreground)
                            .child(format!("{}/{}", players.online, players.max)))
                    } else {
                        this.child(div()
                            .text_color(theme.muted_foreground)
                            .child("???"))
                    }
                })
                .when_some(summary.ping.as_ref(), |this, ping| {
                    let millis = ping.as_millis();

                    let color = if millis < 25 {
                        theme.success
                    } else if millis < 175 {
                        theme.warning.mix_oklab(theme.success, (millis-25) as f32 / 150.0)
                    } else if millis < 575 {
                        theme.danger.mix_oklab(theme.warning, (millis-175) as f32 / 400.0)
                    } else {
                        theme.danger
                    };

                    this.child(div().text_color(color).child(format!("{}ms", millis)))
                })
                .when(summary.status.is_none() && summary.pinging, |this| {
                    this.child(t::instance::quickplay::pinging())
                })
            )
            .when_some(summary.status.as_ref(), |this, status| {
                this.child(div()
                    .whitespace_nowrap()
                    .line_clamp(2)
                    .h(rems(2.0))
                    .text_2xl()
                    .font_family("Minecraft Default")
                    .child(crate::component::create_styled_text(&status.description, false)))
            })
            .when(summary.status.is_none() && !summary.pinging, |this| {
                this.child(div()
                    .whitespace_nowrap()
                    .text_color(theme.danger)
                    .h(rems(2.0))
                    .child(t::instance::quickplay::unable_to_get_status()))
            });

        let id = self.id;
        let name = self.name.clone();
        let data = self.data.clone();
        let target = OsString::from(summary.ip.to_string());
        let row_index = ix.row;

        let move_up = Button::new(("server-up", row_index))
            .compact()
            .small()
            .icon(PandoraIcon::ArrowUp)
            .disabled(!can_reorder || row_index == 0)
            .on_click(cx.listener(move |this, _, _, cx| {
                let delegate = this.delegate_mut();
                *delegate.confirming_delete.lock() = None;
                if !delegate.can_reorder() || row_index == 0 {
                    return;
                }
                delegate.reorder_servers(row_index, row_index.saturating_sub(1), cx);
            }));

        let move_down = Button::new(("server-down", row_index))
            .compact()
            .small()
            .icon(PandoraIcon::ArrowDown)
            .disabled(!can_reorder || row_index + 1 >= self.searched.len())
            .on_click(cx.listener(move |this, _, _, cx| {
                let delegate = this.delegate_mut();
                *delegate.confirming_delete.lock() = None;
                if !delegate.can_reorder() || row_index + 1 >= delegate.searched.len() {
                    return;
                }
                delegate.reorder_servers(row_index, row_index + 1, cx);
            }));

        let hover_group: SharedString = format!("server-row-{row_index}").into();

        // Index in the full list, which differs from the row while searching
        let server_index = self.server_index(row_index);
        let is_confirming_delete = *self.confirming_delete.lock() == Some(row_index);

        let edit_button = Button::new(("server-edit", row_index))
            .compact()
            .small()
            .icon(PandoraIcon::Pencil)
            .tooltip(t::instance::quickplay::edit_server())
            .on_click(cx.listener({
                let name = summary.name.clone();
                let ip = summary.ip.clone();
                move |this, _, window, cx| {
                    let delegate = this.delegate_mut();
                    *delegate.confirming_delete.lock() = None;
                    let Some(index) = server_index else {
                        return;
                    };
                    crate::modals::edit_server::open_edit_server(
                        delegate.id,
                        Some(ExistingServer { index, name: name.clone(), ip: ip.clone() }),
                        delegate.data.backend_handle.clone(),
                        window,
                        cx,
                    );
                    cx.notify();
                }
            }));

        let delete_button = if is_confirming_delete {
            Button::new(("server-delete", row_index))
                .compact()
                .small()
                .danger()
                .icon(PandoraIcon::Check)
                .tooltip(t::instance::quickplay::confirm_delete_server())
                .on_click(cx.listener(move |this, _, _, cx| {
                    let delegate = this.delegate_mut();
                    *delegate.confirming_delete.lock() = None;
                    if let Some(index) = server_index {
                        delegate.data.backend_handle.send(MessageToBackend::DeleteServer {
                            id: delegate.id,
                            index,
                        });
                    }
                    cx.notify();
                }))
        } else {
            Button::new(("server-delete", row_index))
                .compact()
                .small()
                .danger()
                .icon(PandoraIcon::Trash2)
                .tooltip(t::instance::quickplay::delete_server())
                .on_click(cx.listener(move |this, _, _, cx| {
                    *this.delegate_mut().confirming_delete.lock() = Some(row_index);
                    cx.notify();
                }))
        };

        // Arrows on the left, edit and delete on the right. Hidden until the row is
        // hovered, unless a delete is waiting for confirmation
        let actions = v_flex()
            .gap_1()
            .px_2()
            .when(!is_confirming_delete, |this| {
                this.invisible().group_hover(hover_group.clone(), |style| style.visible())
            })
            .child(h_flex().gap_1().child(move_up).child(edit_button))
            .child(h_flex().gap_1().child(move_down).child(delete_button));

        let item = ListItem::new(ix)
            .p_1()
            .child(
                h_flex()
                    .group(hover_group)
                    .gap_1()
                    .child(
                        div()
                            .child(Button::new(ix).success().icon(PandoraIcon::Play).on_click(move |_, window, cx| {
                                root::start_instance(
                                    id,
                                    name.clone(),
                                    Some(QuickPlayLaunch::Multiplayer(target.clone())),
                                    &data,
                                    window,
                                    cx,
                                );
                            }))
                            .px_2(),
                    )
                    .child(icon.size_16().min_w_16().min_h_16())
                    .child(description)
                    .child(actions),
            );

        Some(item)
    }

    fn set_selected_index(&mut self, _ix: Option<IndexPath>, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _cx: &mut Context<ListState<Self>>) -> Task<()> {
        self.search_query = query.to_string();
        self.searched = self.apply_search(query);
        *self.confirming_delete.lock() = None;

        Task::ready(())
    }
}

impl ServersListDelegate {
    fn can_reorder(&self) -> bool {
        self.search_query.is_empty()
    }

    fn set_servers(&mut self, servers: Option<Vec<InstanceServerSummary>>) {
        self.loaded = servers.is_some();
        self.servers = servers.unwrap_or_default();
        self.searched = self.apply_search(&self.search_query);
        *self.confirming_delete.lock() = None;
    }

    fn server_index(&self, row: usize) -> Option<usize> {
        if self.search_query.is_empty() {
            return (row < self.servers.len()).then_some(row);
        }
        self.servers.iter().enumerate()
            .filter(|(_, server)| server.name.contains(&self.search_query))
            .nth(row)
            .map(|(index, _)| index)
    }

    fn apply_search(&self, query: &str) -> Vec<InstanceServerSummary> {
        if query.is_empty() {
            self.servers.clone()
        } else {
            self.servers.iter().filter(|w| w.name.contains(query)).cloned().collect()
        }
    }

    fn reorder_servers(&mut self, from_index: usize, to_index: usize, cx: &mut Context<ListState<Self>>) {
        if !self.can_reorder() {
            return;
        }

        self.data.backend_handle.send(MessageToBackend::ReorderServers {
            id: self.id,
            from_index,
            to_index,
        });
        cx.notify();
    }
}
