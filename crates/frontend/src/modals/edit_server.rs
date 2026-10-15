use std::sync::Arc;

use bridge::{handle::BackendHandle, instance::InstanceID, message::MessageToBackend};
use gpui::{prelude::*, *};
use gpui_component::{WindowExt, button::Button, dialog::Dialog, h_flex, input::{Input, InputEvent, InputState}, v_flex};

pub struct ExistingServer {
    pub index: usize,
    pub name: Arc<str>,
    pub ip: Arc<str>,
}

struct EditServerModalState {
    instance_id: InstanceID,
    backend_handle: BackendHandle,
    existing: Option<ExistingServer>,
    name_input_state: Entity<InputState>,
    address_input_state: Entity<InputState>,
    address_empty: bool,
    default_name: SharedString,
    _address_input_subscription: Subscription,
}

impl EditServerModalState {
    fn new(
        instance_id: InstanceID,
        backend_handle: BackendHandle,
        existing: Option<ExistingServer>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let default_name = SharedString::from(t::instance::quickplay::default_server_name());

        let name_input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(default_name.clone());
            if let Some(existing) = &existing {
                state.set_value(existing.name.to_string(), window, cx);
            }
            state
        });
        let address_input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("play.example.com");
            if let Some(existing) = &existing {
                state.set_value(existing.ip.to_string(), window, cx);
            }
            state
        });

        let address_empty = existing.as_ref().map_or(true, |existing| existing.ip.trim().is_empty());
        let _address_input_subscription = cx.subscribe_in(&address_input_state, window, |this, input_state, _: &InputEvent, _, cx| {
            this.address_empty = input_state.read(cx).value().trim().is_empty();
            cx.notify();
        });

        Self {
            instance_id,
            backend_handle,
            existing,
            name_input_state,
            address_input_state,
            address_empty,
            default_name,
            _address_input_subscription,
        }
    }

    fn render(&mut self, dialog: Dialog, _window: &mut Window, cx: &mut Context<Self>) -> Dialog {
        let editing = self.existing.is_some();
        let title = if editing {
            t::instance::quickplay::edit_server()
        } else {
            t::instance::quickplay::add_server()
        };

        let content = v_flex()
            .gap_3()
            .child(crate::labelled(t::instance::name(), Input::new(&self.name_input_state)))
            .child(crate::labelled(t::instance::quickplay::server_address(), Input::new(&self.address_input_state)));

        let cancel = Button::new("cancel").flex_1().label(t::common::cancel())
            .on_click(|_, window, cx| window.close_dialog(cx));

        let ok = if self.address_empty {
            Button::new("ok").flex_1().opacity(0.5).label(t::common::ok())
        } else {
            Button::new("ok").flex_1().label(t::common::ok())
                .on_click(cx.listener(move |this, _, window, cx| {
                    let mut name = this.name_input_state.read(cx).value().trim().to_string();
                    if name.is_empty() {
                        name = this.default_name.to_string();
                    }
                    let ip: Arc<str> = this.address_input_state.read(cx).value().trim().into();
                    let name: Arc<str> = name.into();

                    if let Some(existing) = &this.existing {
                        this.backend_handle.send(MessageToBackend::EditServer {
                            id: this.instance_id,
                            index: existing.index,
                            name,
                            ip,
                        });
                    } else {
                        this.backend_handle.send(MessageToBackend::AddServer {
                            id: this.instance_id,
                            name,
                            ip,
                        });
                    }

                    window.close_dialog(cx);
                }))
        };

        dialog
            .overlay_closable(false)
            .title(title)
            .child(content)
            .footer(h_flex().gap_2().w_full().child(cancel).child(ok))
    }
}

pub fn open_edit_server(
    instance_id: InstanceID,
    existing: Option<ExistingServer>,
    backend_handle: BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let state = cx.new(|cx| EditServerModalState::new(instance_id, backend_handle, existing, window, cx));

    window.open_dialog(cx, move |modal, window, cx| {
        cx.update_entity(&state, |state, cx| state.render(modal, window, cx))
    });
}
