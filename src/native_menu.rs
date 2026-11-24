//! The macOS menu bar: File, Edit, View and Help in the system bar at the top
//! of the screen, next to the application menu, instead of inside the window.
//!
//! Items carry no accelerator: the shortcuts are configurable and already
//! handled by the app's own key handling, a native accelerator would fire
//! them twice. Choosing an item sends the same `Message` as its in-window
//! counterpart on the other platforms.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::futures::{SinkExt, Stream};
use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::mpsc;

use crate::app::{Message, TopMenu};
use crate::config::Config;
use crate::menu_bar::BookmarkState;
use crate::menus::{self, Entry, MenuState};

/// What the menu shows; it is rebuilt when any of it changes.
pub struct State<'a> {
    pub config: &'a Config,
    pub has_formatter: bool,
    pub lang_picker_enabled: bool,
    pub bookmarks: BookmarkState,
    pub tools: crate::plugins::Tools,
    pub split: bool,
}

impl State<'_> {
    /// Everything the menu's labels and enabled states depend on.
    pub fn signature(&self) -> String {
        let c = self.config;
        format!(
            "{}|{}|{}|{:?}|{}|{}|{}|{}|{:?}|{}",
            &*rust_i18n::locale(),
            c.dark_mode,
            c.show_line_numbers,
            c.recent_files,
            self.has_formatter,
            self.lang_picker_enabled,
            self.bookmarks.on_line,
            self.bookmarks.any,
            self.tools,
            self.split,
        )
    }
}

type Actions = HashMap<MenuId, Message>;

static ACTIONS: OnceLock<Mutex<Actions>> = OnceLock::new();
static EVENTS: OnceLock<Mutex<Option<mpsc::UnboundedReceiver<MenuId>>>> = OnceLock::new();

fn actions() -> &'static Mutex<Actions> {
    ACTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The messages of the items the user chooses. Runs for the life of the app.
pub fn subscription() -> iced::Subscription<Message> {
    iced::Subscription::run(events)
}

fn events() -> impl Stream<Item = Message> {
    iced::stream::channel(32, async |mut output| {
        let receiver = channel().lock().ok().and_then(|mut r| r.take());
        let Some(mut receiver) = receiver else {
            return std::future::pending().await;
        };
        while let Some(id) = receiver.recv().await {
            let message = actions().lock().ok().and_then(|a| a.get(&id).cloned());
            if let Some(message) = message {
                let _ = output.send(message).await;
            }
        }
    })
}

/// The channel the menu events go through, made by whoever comes first: the
/// subscription starts before the menu is installed.
fn channel() -> &'static Mutex<Option<mpsc::UnboundedReceiver<MenuId>>> {
    EVENTS.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = tx.send(event.id);
        }));
        Mutex::new(Some(rx))
    })
}

struct Builder {
    actions: Actions,
}

impl Builder {
    fn add(&mut self, menu: &Submenu, label: impl AsRef<str>, enabled: bool, message: Message) {
        let item = MenuItem::new(label, enabled, None);
        self.actions.insert(item.id().clone(), message);
        let _ = menu.append(&item);
    }

    fn on(&mut self, menu: &Submenu, label: impl AsRef<str>, message: Message) {
        self.add(menu, label, true, message);
    }
}

fn separator(menu: &Submenu) {
    let _ = menu.append(&PredefinedMenuItem::separator());
}

/// Installs the menu bar, replacing the previous one.
pub fn install(state: &State) {
    channel();
    let mut b = Builder {
        actions: HashMap::new(),
    };
    let menu = Menu::new();

    let _ = menu.append(&app_menu(&mut b));
    let _ = menu.append(&menu_of(
        &mut b,
        t!("menu.file").to_string(),
        TopMenu::File,
        state,
    ));
    let _ = menu.append(&menu_of(
        &mut b,
        t!("menu.edit").to_string(),
        TopMenu::Edit,
        state,
    ));
    let _ = menu.append(&menu_of(
        &mut b,
        t!("menu.view").to_string(),
        TopMenu::View,
        state,
    ));
    let help = menu_of(&mut b, t!("menu.help").to_string(), TopMenu::Help, state);
    let _ = menu.append(&help);

    if let Ok(mut a) = actions().lock() {
        *a = b.actions;
    }
    menu.init_for_nsapp();
    help.set_as_help_menu_for_nsapp();
    // The menu must outlive this call: the system holds it by reference.
    thread_local!(static KEEP: std::cell::RefCell<Option<Menu>> = const { std::cell::RefCell::new(None) });
    KEEP.with(|keep| *keep.borrow_mut() = Some(menu));
}

/// The first menu, named after the app by the system.
fn app_menu(b: &mut Builder) -> Submenu {
    let menu = Submenu::new("Ardoise", true);
    b.on(&menu, t!("menu.help_about"), Message::About);
    separator(&menu);
    b.on(
        &menu,
        t!("menu.file_preferences"),
        Message::TogglePreferences,
    );
    separator(&menu);
    let _ = menu.append(&PredefinedMenuItem::services(None));
    separator(&menu);
    let _ = menu.append(&PredefinedMenuItem::hide(None));
    let _ = menu.append(&PredefinedMenuItem::hide_others(None));
    let _ = menu.append(&PredefinedMenuItem::show_all(None));
    separator(&menu);
    // Not the system's Quit: it must go through the app to save the session.
    b.on(&menu, t!("menu.file_quit"), Message::Quit);
    menu
}

/// The entries of a menu, with their submenus, in the system's.
fn append(b: &mut Builder, menu: &Submenu, entries: &[Entry]) {
    for entry in entries {
        match entry {
            Entry::Item {
                label,
                enabled,
                message,
                ..
            } => b.add(menu, label, *enabled, message.clone()),
            Entry::Separator => separator(menu),
            Entry::Sub {
                label,
                enabled,
                entries,
                ..
            } => {
                let sub = Submenu::new(label, *enabled);
                append(b, &sub, entries);
                let _ = menu.append(&sub);
            }
        }
    }
}

fn menu_of(b: &mut Builder, title: String, top: TopMenu, state: &State) -> Submenu {
    let menu = Submenu::new(title, true);
    let shown = MenuState {
        config: state.config,
        has_formatter: state.has_formatter,
        lang_picker_enabled: state.lang_picker_enabled,
        bookmarks: state.bookmarks,
        tools: state.tools,
        split: state.split,
    };
    append(b, &menu, &menus::entries(top, &shown, true));
    menu
}
