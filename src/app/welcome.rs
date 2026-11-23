//! The popin of the first launch: the few settings worth asking, then the
//! official plugins to install in one click. It shows once, on a fresh
//! install (see `announce_new_version`).

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

use super::{ArdoiseApp, Message};
use crate::plugins::{PluginMsg, Registry};
use crate::popin::{self, Action};
use crate::theme as se_theme;

impl ArdoiseApp {
    pub(super) fn view_welcome_overlay(&self, dark: bool) -> Element<'_, Message> {
        let p = se_theme::palette(dark);
        let note = move |said: String| text(said).size(12).color(p.muted);
        let heading = move |said: String| {
            text(said)
                .size(13)
                .font(se_theme::UI_SEMIBOLD)
                .color(p.text)
        };
        let body = column![
            note(t!("welcome.intro").to_string()),
            crate::preferences::basics(&self.config, dark),
            column![
                heading(t!("welcome.plugins_title").to_string()),
                note(t!("welcome.plugins_hint").to_string()),
            ]
            .spacing(2),
            self.welcome_plugins(dark),
            note(t!("welcome.later").to_string()),
        ]
        .spacing(16)
        .width(Length::Fill);
        popin::card(
            dark,
            popin::WIDTH_FORM,
            t!("welcome.title").to_string(),
            Some(Message::CloseOverlay),
            container(
                scrollable(container(body).padding(iced::Padding {
                    right: 14.0,
                    ..iced::Padding::ZERO
                }))
                .style(se_theme::Scrollbar { dark }.style()),
            )
            .max_height(popin::MAX_BODY)
            .into(),
            Some((
                vec![],
                vec![popin::action(
                    dark,
                    t!("welcome.done").to_string(),
                    Some(Message::CloseOverlay),
                    Action::Primary,
                )],
            )),
        )
    }

    /// The official plugins, each with its button.
    fn welcome_plugins(&self, dark: bool) -> Element<'_, Message> {
        let p = se_theme::palette(dark);
        let note = move |said: String| text(said).size(12).color(p.muted);
        let locale = rust_i18n::locale();
        match &self.plugins.registry {
            Registry::Unknown | Registry::Loading => {
                note(t!("plugins.loading_list").to_string()).into()
            }
            Registry::Failed(e) => row![
                text(t!("plugins.err_list", error = e.as_str()).to_string())
                    .size(12)
                    .color(se_theme::error_color(dark)),
                Space::new().width(Length::Fill),
                popin::action(
                    dark,
                    t!("plugins.retry").to_string(),
                    Some(Message::Plugin(PluginMsg::LoadRegistry)),
                    Action::Cancel,
                ),
            ]
            .align_y(Alignment::Center)
            .into(),
            Registry::Loaded(list) => crate::preferences::card_of_rows(
                dark,
                list.iter()
                    .map(|listed| {
                        let installed = self.plugins.get(&listed.id).is_some();
                        let control: Element<Message> = if self.plugins.busy.contains(&listed.id) {
                            note(t!("plugins.working").to_string()).into()
                        } else if installed {
                            note(t!("welcome.installed").to_string()).into()
                        } else {
                            button(text(t!("plugins.install").to_string()).size(12))
                                .padding([6, 12])
                                .on_press(Message::Plugin(PluginMsg::InstallOfficial(
                                    listed.id.clone(),
                                )))
                                .style((se_theme::Primary { dark }).style())
                                .into()
                        };
                        let mut label = column![text(listed.name.get(&locale).to_string())
                            .size(13)
                            .shaping(iced::widget::text::Shaping::Advanced)]
                        .spacing(2);
                        let description = listed.description.get(&locale);
                        if !description.is_empty() {
                            label = label.push(note(description.to_string()));
                        }
                        if let Some(e) = self.plugins.errors.get(&listed.id) {
                            label = label
                                .push(text(e.clone()).size(12).color(se_theme::error_color(dark)));
                        }
                        row![label.width(Length::Fill), control]
                            .spacing(10)
                            .align_y(Alignment::Center)
                            .padding([12, 16])
                            .into()
                    })
                    .collect(),
            ),
        }
    }
}
