//! Preferences › Plugins: the official plugins to install or remove, the
//! plugins of others (from their GitHub repository), the updates, and a
//! folder to link while writing one.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

use super::{card_of_rows, setting_row};
use crate::app::Message;
use crate::plugins::{PluginMsg, Plugins, Registry};
use crate::theme;

fn msg(m: PluginMsg) -> Message {
    Message::Plugin(m)
}

fn small_button(
    label: String,
    on: Option<Message>,
    dark: bool,
    primary: bool,
) -> Element<'static, Message> {
    let b = button(text(label).size(12))
        .padding([6, 12])
        .on_press_maybe(on);
    if primary {
        b.style((theme::Primary { dark }).style()).into()
    } else {
        b.style((theme::Outline { dark }).style()).into()
    }
}

/// A plugin of a list: its label and its controls. Its settings are a page of
/// their own in the preferences.
fn entry<'a>(
    label: iced::widget::Column<'a, Message>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![label.width(Length::Fill), control]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([12, 16])
        .into()
}

pub(super) fn page<'a>(plugins: &'a Plugins, dark: bool) -> Element<'a, Message> {
    let p = theme::palette(dark);
    let note = move |label: String| text(label).size(12).color(p.muted);
    let error = move |label: String| text(label).size(12).color(theme::error_color(dark));
    let locale = rust_i18n::locale();

    // ── The official plugins ──────────────────────────────────────────
    let official: Element<Message> = match &plugins.registry {
        Registry::Unknown | Registry::Loading => {
            note(t!("plugins.loading_list").to_string()).into()
        }
        Registry::Failed(e) => row![
            error(t!("plugins.err_list", error = e.as_str()).to_string()),
            Space::new().width(Length::Fill),
            small_button(
                t!("plugins.retry").to_string(),
                Some(msg(PluginMsg::LoadRegistry)),
                dark,
                false
            ),
        ]
        .align_y(Alignment::Center)
        .into(),
        Registry::Loaded(list) => card_of_rows(
            dark,
            list.iter()
                .map(|listed| {
                    let installed = plugins.get(&listed.id);
                    let busy = plugins.busy.contains(&listed.id);
                    let control: Element<Message> = if busy {
                        note(t!("plugins.working").to_string()).into()
                    } else if let Some(installed) = installed {
                        let v = installed.manifest.version.as_str();
                        let mut r = row![note(
                            t!("plugins.installed_version", version = v).to_string()
                        )]
                        .spacing(10)
                        .align_y(Alignment::Center);
                        if let Some(newer) = plugins.updates.get(&listed.id) {
                            r = r.push(small_button(
                                t!("plugins.update_to", version = newer.as_str()).to_string(),
                                Some(msg(PluginMsg::InstallOfficial(listed.id.clone()))),
                                dark,
                                true,
                            ));
                        }
                        r.push(small_button(
                            t!("plugins.uninstall").to_string(),
                            Some(msg(PluginMsg::AskUninstall(listed.id.clone()))),
                            dark,
                            false,
                        ))
                        .into()
                    } else {
                        small_button(
                            t!("plugins.install").to_string(),
                            Some(msg(PluginMsg::InstallOfficial(listed.id.clone()))),
                            dark,
                            false,
                        )
                    };
                    let mut label =
                        column![text(listed.name.get(&locale).to_string()).size(13)].spacing(2);
                    let description = listed.description.get(&locale);
                    if !description.is_empty() {
                        label = label.push(note(description.to_string()));
                    }
                    if let Some(e) = plugins.errors.get(&listed.id) {
                        label = label.push(error(e.clone()));
                    }
                    entry(label, control)
                })
                .collect(),
        ),
    };

    // ── The other plugins installed ───────────────────────────────────
    let listed = |id: &str| match &plugins.registry {
        Registry::Loaded(list) => list.iter().any(|l| l.id == id),
        _ => false,
    };
    let others: Vec<Element<Message>> = plugins
        .installed
        .iter()
        .filter(|i| !(i.origin.official && listed(i.id())))
        .map(|installed| {
            let id = installed.id().to_string();
            let source = if let Some(folder) = &installed.origin.linked {
                let f = folder.display().to_string();
                t!("plugins.linked", folder = f.as_str()).to_string()
            } else if let Some(repo) = &installed.origin.repository {
                repo.clone()
            } else {
                String::new()
            };
            let mut label = column![row![
                text(installed.name()).size(13),
                note(format!("v{}", installed.manifest.version)),
            ]
            .spacing(8)]
            .spacing(2);
            let description = installed.description();
            if !description.is_empty() {
                label = label.push(note(description));
            }
            if !source.is_empty() {
                label = label.push(note(source).font(theme::MONO));
            }
            if let Some(e) = plugins.errors.get(&id) {
                label = label.push(error(e.clone()));
            }
            let mut control = row![].spacing(10).align_y(Alignment::Center);
            if plugins.busy.contains(&id) {
                control = control.push(note(t!("plugins.working").to_string()));
            } else {
                if let Some(newer) = plugins.updates.get(&id) {
                    control = control.push(small_button(
                        t!("plugins.update_to", version = newer.as_str()).to_string(),
                        Some(msg(PluginMsg::Update(id.clone()))),
                        dark,
                        true,
                    ));
                }
                control = control.push(small_button(
                    t!("plugins.uninstall").to_string(),
                    Some(msg(PluginMsg::AskUninstall(id.clone()))),
                    dark,
                    false,
                ));
            }
            entry(label, control.into())
        })
        .chain(plugins.broken.iter().map(|(name, why)| {
            setting_row(
                name.clone(),
                error(why.clone()).width(Length::FillPortion(2)).into(),
            )
        }))
        .collect();

    // ── A plugin of someone else ──────────────────────────────────────
    let fetching = plugins.busy.contains("repo");
    let field = text_input(&t!("plugins.repo_hint"), &plugins.repo_input)
        .on_input(|v| msg(PluginMsg::RepoTyped(v)))
        .on_submit(msg(PluginMsg::FetchRepo))
        .padding([6, 10])
        .size(13)
        .width(Length::Fill)
        .style(theme::Field::new(dark).style());
    let mut add = column![row![
        field,
        small_button(
            t!("plugins.fetch").to_string(),
            (!fetching).then(|| msg(PluginMsg::FetchRepo)),
            dark,
            false
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center),]
    .spacing(8)
    .padding([12, 16]);
    if fetching {
        add = add.push(note(t!("plugins.working").to_string()));
    }
    if let Some(e) = plugins.errors.get("repo") {
        add = add.push(error(e.clone()));
    }
    if let Some(review) = &plugins.review {
        let m = &review.manifest;
        let mut what = Vec::new();
        for c in &m.contributes.commands {
            what.push(t!("plugins.adds_command", name = c.title.get(&locale)).to_string());
        }
        for panel in &m.contributes.panels {
            what.push(t!("plugins.adds_panel", name = panel.title.get(&locale)).to_string());
        }
        if m.starts_with_editor() {
            what.push(t!("plugins.adds_startup").to_string());
        }
        let mut card = column![
            row![
                text(m.name.get(&locale).to_string())
                    .size(14)
                    .font(theme::UI_SEMIBOLD),
                note(format!("v{}", m.version)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            note(format!(
                "{} · {}",
                m.author
                    .clone()
                    .unwrap_or_else(|| t!("plugins.unknown_author").to_string()),
                review.repository
            )),
        ]
        .spacing(6);
        let description = m.description.get(&locale);
        if !description.is_empty() {
            card = card.push(text(description.to_string()).size(12.5));
        }
        for line in what {
            card = card.push(note(format!("• {line}")));
        }
        card = card.push(error(t!("plugins.warning").to_string())).push(
            row![
                small_button(
                    t!("confirm.cancel").to_string(),
                    Some(msg(PluginMsg::CancelReview)),
                    dark,
                    false
                ),
                Space::new().width(Length::Fill),
                small_button(
                    t!("plugins.install_anyway").to_string(),
                    Some(msg(PluginMsg::AcceptReview)),
                    dark,
                    true
                ),
            ]
            .align_y(Alignment::Center),
        );
        add = add.push(
            container(card.padding(14))
                .width(Length::Fill)
                .style(theme::card(dark)),
        );
    }

    let mut tools = row![
        small_button(
            if plugins.checking_updates {
                t!("plugins.checking").to_string()
            } else {
                t!("plugins.check_updates").to_string()
            },
            (!plugins.checking_updates).then(|| msg(PluginMsg::CheckUpdates)),
            dark,
            false,
        ),
        small_button(
            t!("plugins.guide").to_string(),
            Some(msg(PluginMsg::OpenGuide)),
            dark,
            false
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if cfg!(unix) {
        tools = tools.push(small_button(
            t!("plugins.link").to_string(),
            Some(msg(PluginMsg::LinkFolder)),
            dark,
            false,
        ));
    }

    let heading = |label: String| text(label).size(13).font(theme::UI_SEMIBOLD);
    let mut page = column![
        note(t!("plugins.intro").to_string()),
        tools,
        heading(t!("plugins.official").to_string()),
        official,
    ]
    .spacing(12);
    if !others.is_empty() {
        page = page
            .push(heading(t!("plugins.others").to_string()))
            .push(card_of_rows(dark, others));
    }
    page.push(heading(t!("plugins.add_other").to_string()))
        .push(
            container(add)
                .width(Length::Fill)
                .style(theme::settings_card(dark)),
        )
        .into()
}
