/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]

//! Shrug is a small program where you can have a library of named strings.  You can then search for
//! those strings to have them readily available in your clipboard.
//!
//! This is what it looks like:
//!
//! <p align="center">
//! <img src="https://raw.githubusercontent.com/orium/shrug/main/images/shrug.png" width="300">
//! </p>
//!
//! I suggest you add a key binding in your window manager to launch shrug.
//!
//! Note that shrug keeps running in the background after being launched.  This is because in X.org,
//! the clipboard content belongs to the program the content originated from.  If the program
//! terminates the content of the clipboard gets cleared.  (An alternative would be to use a
//! clipboard manager ¯\\\_(ツ)\_/¯.)

mod config;
mod text_clipboard;

use crate::text_clipboard::TextClipboard;
use config::Alias;
use config::Config;
use gdk::Key;
use gdk4 as gdk;
use gdk4::glib::Type;
use gio::prelude::*;
use glib::Propagation;
use glib::{ControlFlow, Priority};
use gtk::prelude::*;
use gtk4 as gtk;
use rayon::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sublime_fuzzy::FuzzySearch;

fn paste_and_hide(
    window: &gtk::Window,
    tree_view: &gtk::TreeView,
    sorted_store: &gtk::TreeModelSort,
    search_entry: &gtk::SearchEntry,
) {
    if let Some((_, row)) = tree_view.selection().selected() {
        let str: String = tree_view.model().unwrap().get_value(&row, 1).get().unwrap();

        TextClipboard::new(&gdk::Display::default().unwrap()).set(&str);
    }

    hide(window, search_entry, tree_view, sorted_store);
}

fn hide(
    window: &gtk::Window,
    search_entry: &gtk::SearchEntry,
    tree_view: &gtk::TreeView,
    sorted_store: &gtk::TreeModelSort,
) {
    window.hide();

    search_entry.set_text("");

    if let Some(first_row) = sorted_store.iter_first() {
        tree_view.selection().select_iter(&first_row);
    }
}

// WIP! review and organize.
fn build_ui(app: &gtk::Application, config: Config, show_listener: Arc<UnixListener>) {
    let glade_src = include_str!("window_main.ui");

    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.object("window_main").unwrap();

    window.set_application(Some(app));

    let tree_view: gtk::TreeView = builder.object("tree_view").unwrap();

    {
        let renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&renderer, true); // WIP! NEEDED?
        column.add_attribute(&renderer, "text", 0);
        column.set_sort_order(gtk::SortType::Descending);
        tree_view.append_column(&column);
    }

    {
        let renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&renderer, true);
        column.add_attribute(&renderer, "text", 1);
        tree_view.append_column(&column);
    }

    {
        let column = gtk::TreeViewColumn::new();
        column.set_title("Value");
        column.set_clickable(true); // WIP! get rid of these
        column.set_sort_indicator(true);
        column.set_sort_column_id(2); // WIP! gtk::SortType::Descending
        tree_view.append_column(&column);
    }

    let store: gtk::TreeStore = gtk::TreeStore::new(&[Type::STRING, Type::STRING, Type::I64]);

    for alias in config.aliases() {
        store.set(&store.append(None), &[(0, &alias.key), (1, &alias.value), (2, &0i64)]);
    }

    let sorted_store: gtk::TreeModelSort = gtk::TreeModelSort::with_model(&store);

    sorted_store.set_sort_column_id(gtk::SortColumn::Index(2), gtk::SortType::Descending);

    tree_view.set_model(Some(&sorted_store));

    if let Some(first_row) = sorted_store.iter_first() {
        tree_view.selection().select_iter(&first_row);
    }

    let search_entry: gtk::SearchEntry = builder.object("search_entry").unwrap();

    // WIP! can we do this on every change?
    search_entry.connect_search_changed({
        let tree_view = tree_view.clone();
        let sorted_store = sorted_store.clone();

        move |entry| {
            let text = entry.text().to_string();
            let mut scored_aliases: Vec<(Alias, i64)> = config
                .aliases()
                .par_bridge()
                .map(|alias| {
                    let score: i64 = FuzzySearch::new(&text, alias.key)
                        .best_match()
                        .map_or(0, |m| m.score() as i64);

                    (alias, score)
                })
                .collect();

            scored_aliases.sort_by_key(|(alias, score)| (*score, alias.key));

            // TODO Do not clear each time.
            store.clear();

            scored_aliases.iter().for_each(|(alias, score)| {
                store.set(&store.append(None), &[(0, &alias.key), (1, &alias.value), (2, &score)]);
            });

            tree_view.selection().unselect_all();

            if let Some(first_row) = sorted_store.iter_first() {
                tree_view.selection().select_iter(&first_row);
            }
        }
    });

    let (tx, rx) = glib::MainContext::channel(Priority::DEFAULT);

    std::thread::spawn(move || {
        for _ in show_listener.incoming() {
            tx.send(()).unwrap();
        }
    });

    rx.attach(None, {
        let window = window.clone();

        move |()| {
            window.show();
            ControlFlow::Continue
        }
    });

    let key_press_controller: gtk::EventControllerKey = {
        let window = window.clone();
        let controller = gtk::EventControllerKey::new();

        // For whatever reason these keys can only be observed in the "release_key" event.
        controller.connect_key_released({
            let tree_view = tree_view.clone();

            move |_, key, _, _| match key {
                Key::Return => {
                    paste_and_hide(&window, &tree_view, &sorted_store, &search_entry);
                }
                Key::Escape => {
                    hide(&window, &search_entry, &tree_view, &sorted_store);
                }
                _ => (),
            }
        });

        controller.connect_key_pressed(move |_, key, _, _| {
            fn move_selection(tree_view: &gtk::TreeView, up: bool) {
                let selection = tree_view.selection();

                if let Some((tree_model, tree_iter)) = selection.selected() {
                    let moved = match up {
                        true => tree_model.iter_previous(&tree_iter),
                        false => tree_model.iter_next(&tree_iter),
                    };

                    if moved {
                        selection.select_iter(&tree_iter);
                    }
                }
            }

            match key {
                Key::Up => {
                    move_selection(&tree_view, true);
                    Propagation::Stop
                }
                Key::Down => {
                    move_selection(&tree_view, false);
                    Propagation::Stop
                }
                _ => Propagation::Proceed,
            }
        });

        controller
    };

    window.add_controller(key_press_controller);

    window.show();
}

fn launch_application(show_listener: UnixListener) {
    let application = gtk::Application::new(None::<&str>, gio::ApplicationFlags::default());
    let show_listener: Arc<UnixListener> = Arc::new(show_listener);

    application.connect_activate(move |app| {
        Config::create_file_if_nonexistent();

        let config: Config = Config::from_config_file();

        build_ui(app, config, Arc::clone(&show_listener));
    });

    application.run();
}

enum SendShowSignalOrListen {
    Listener(UnixListener),
    SignalSent,
}

fn send_show_signal_or_listen(
    socket_path: impl AsRef<Path>,
) -> Result<SendShowSignalOrListen, std::io::Error> {
    use std::io::ErrorKind;

    match UnixListener::bind(&socket_path) {
        Ok(listener) => Ok(SendShowSignalOrListen::Listener(listener)),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            // We don't know if there's another shrug listening there or not.  Let's find out.
            match UnixStream::connect(&socket_path) {
                Ok(_) => Ok(SendShowSignalOrListen::SignalSent),
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    // There's no one there, we should be able to get bind now.
                    std::fs::remove_file(&socket_path)?;
                    Ok(SendShowSignalOrListen::Listener(UnixListener::bind(&socket_path)?))
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

fn unix_socket_show_signal_path() -> PathBuf {
    dirs::runtime_dir().expect("no runtime dir").join("shrug_show.sock")
}

fn main() {
    match send_show_signal_or_listen(unix_socket_show_signal_path()) {
        Ok(SendShowSignalOrListen::Listener(listener)) => {
            launch_application(listener);
        }
        Ok(SendShowSignalOrListen::SignalSent) => {
            // We sent the signal.  Nothing else to do now.
            println!("sent signal to running shrug.");
        }
        Err(e) => {
            eprintln!("error: failed to send signal: {e}");
        }
    }
}

/* TODO
 *
 * config deamonize
 * Organize code
 * Proper error handling in config
 * Proper error handling in clipboard?
 * Proper error: no unwraps
 */
