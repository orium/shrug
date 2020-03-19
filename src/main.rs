/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]

mod config;
mod text_clipboard;

use crate::text_clipboard::TextClipboard;
use config::Alias;
use config::Config;
use gdk::enums::key;
use gio::prelude::*;
use gtk::prelude::*;
use rayon::prelude::*;
use signal_hook::iterator::Signals;
use std::env::args;
use sublime_fuzzy::FuzzySearch;
use psutil::process::Signal;

// WIP! review and organize.
fn build_ui(app: &gtk::Application, config: Config) {
    let glade_src = include_str!("window_main.glade");

    let builder = gtk::Builder::new_from_string(glade_src);

    let window: gtk::Window = builder.get_object("window_main").unwrap();

    window.set_keep_above(true);
    window.set_application(Some(app));

    let tree_view: gtk::TreeView = builder.get_object("tree_view").unwrap();

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

    let store: gtk::TreeStore =
        gtk::TreeStore::new(&[glib::Type::String, glib::Type::String, glib::Type::I64]);

    for alias in config.aliases() {
        store.set(&store.append(None), &[0, 1, 2], &[&alias.key, &alias.value, &0]);
    }

    let sorted_store: gtk::TreeModelSort = gtk::TreeModelSort::new(&store);

    sorted_store.set_sort_column_id(gtk::SortColumn::Index(2), gtk::SortType::Descending);

    tree_view.set_model(Some(&sorted_store));

    if let Some(first_row) = sorted_store.get_iter_first() {
        tree_view.get_selection().select_iter(&first_row);
    }

    let search_entry: gtk::SearchEntry = builder.get_object("search_entry").unwrap();

    let tree_view_clone = tree_view.clone();

    // WIP! can we do this on every change?
    search_entry.connect_search_changed(move |entry| {
        if let Some(text) = entry.get_text().map(|s| s.to_string()) {
            let mut scored_aliases: Vec<(Alias, i64)> = config
                .aliases()
                .par_bridge()
                .map(|alias| {
                    let score: i64 = FuzzySearch::new(&text, alias.key, false)
                        .best_match()
                        .map(|m| m.score() as i64)
                        .unwrap_or(0);

                    (alias, score)
                })
                .collect();

            scored_aliases.sort_by_key(|(alias, score)| (*score, alias.key));

            // TODO Do not clear each time.
            store.clear();

            scored_aliases.iter().for_each(|(alias, score)| {
                store.set(&store.append(None), &[0, 1, 2], &[&alias.key, &alias.value, &score]);
            });

            tree_view.get_selection().unselect_all();

            if let Some(first_row) = sorted_store.get_iter_first() {
                tree_view.get_selection().select_iter(&first_row);
            }
        }
    });

    fn paste_and_hide(
        window: &gtk::Window,
        tree_view: &gtk::TreeView,
        search_entry: &gtk::SearchEntry,
    ) {
        if let Some((_, row)) = tree_view.get_selection().get_selected() {
            let str: String = tree_view
                .get_model()
                .unwrap()
                .get_value(&row, 1)
                .downcast()
                .unwrap()
                .get()
                .unwrap();

            TextClipboard::new().set(&str);
        }

        hide(window, search_entry);
    };

    fn hide(window: &gtk::Window, search_entry: &gtk::SearchEntry) {
        window.hide();

        search_entry.set_text("");
        // WIP! reset list
    }

    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    std::thread::spawn(move || {
        let signals = Signals::new(&[signal_hook::SIGUSR1]).unwrap();

        for _ in signals.forever() {
            tx.send(()).unwrap();
        }
    });

    let window_clone: gtk::Window = window.clone();

    rx.attach(None, move |_| {
        window_clone.show_all();
        glib::Continue(true)
    });

    window.connect_key_press_event(move |window, key_event| {
        fn move_selection(tree_view: &gtk::TreeView, up: bool) {
            let selection = tree_view.get_selection();
            match selection.get_selected() {
                Some((tree_model, tree_iter)) => {
                    let moved = match up {
                        true => tree_model.iter_previous(&tree_iter),
                        false => tree_model.iter_next(&tree_iter),
                    };

                    if moved {
                        selection.select_iter(&tree_iter);
                    }
                }
                None => (),
            }
        }

        match key_event.get_keyval() {
            key::Return => {
                paste_and_hide(&window, &tree_view_clone, &search_entry);
                Inhibit(true)
            }
            key::Escape => {
                hide(&window, &search_entry);
                Inhibit(true)
            }
            key::Up => {
                move_selection(&tree_view_clone, true);
                Inhibit(true)
            }
            key::Down => {
                move_selection(&tree_view_clone, false);
                Inhibit(true)
            }
            _ => Inhibit(false),
        }
    });

    window.show_all();
}

fn launch_application() {
    let application = gtk::Application::new(None, Default::default())
        .expect("Failed initializing gtk application");

    application.connect_activate(|app| {
        Config::create_file_if_nonexistent();

        let config: Config = Config::from_config_file();

        build_ui(app, config);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn main() {
    let my_pid = std::process::id();

    for process in psutil::process::processes().expect("Failed to get process list") {
        let process = match process {
            Ok(p) => p,
            Err(_) => continue,
        };

        if let Ok(Some(cmd)) = process.cmdline_vec() {
            if my_pid != process.pid() && cmd.first().map_or(false, |s| s.ends_with("shrug")) {
                process.send_signal(Signal::SIGUSR1).expect("Failed to send SIGUSR1 signal");
                return;
            }
        }
    }

    launch_application();
}

/* WIP!
 *
 * config deamonize
 * Organize code
 * Proper error handling in config
 * Proper error handling in clipboard?
 * Proper error: no unwraps
 * README
 * make portable in terms of signals
 * travis
 */
