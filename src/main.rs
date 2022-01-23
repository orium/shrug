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
use gdk::Key;
use gdk4 as gdk;
use gio::prelude::*;
use glib::signal::Inhibit;
use gtk::prelude::*;
use gtk4 as gtk;
use rayon::prelude::*;
use signal_hook::iterator::Signals;
use sublime_fuzzy::FuzzySearch;

// WIP! review and organize.
fn build_ui(app: &gtk::Application, config: Config) {
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

    let store: gtk::TreeStore =
        gtk::TreeStore::new(&[glib::Type::STRING, glib::Type::STRING, glib::Type::I64]);

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

        move |entry| {
            let text = entry.text().to_string();
            let mut scored_aliases: Vec<(Alias, i64)> = config
                .aliases()
                .par_bridge()
                .map(|alias| {
                    let score: i64 = FuzzySearch::new(&text, alias.key)
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
                store.set(&store.append(None), &[(0, &alias.key), (1, &alias.value), (2, &score)]);
            });

            tree_view.selection().unselect_all();

            if let Some(first_row) = sorted_store.iter_first() {
                tree_view.selection().select_iter(&first_row);
            }
        }
    });

    fn paste_and_hide(
        window: &gtk::Window,
        tree_view: &gtk::TreeView,
        search_entry: &gtk::SearchEntry,
    ) {
        if let Some((_, row)) = tree_view.selection().selected() {
            let str: String = tree_view.model().unwrap().get_value(&row, 1).get().unwrap();

            TextClipboard::new(&gdk::Display::default().unwrap()).set(&str);
        }

        hide(window, search_entry);
    }

    fn hide(window: &gtk::Window, search_entry: &gtk::SearchEntry) {
        window.hide();

        search_entry.set_text("");
        // WIP! reset list
    }

    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    std::thread::spawn(move || {
        use signal_hook::consts::SIGUSR1;

        let mut signals = Signals::new(&[SIGUSR1]).unwrap();

        for _ in signals.forever() {
            tx.send(()).unwrap();
        }
    });

    rx.attach(None, {
        let window = window.clone();

        move |_| {
            window.show();
            glib::Continue(true)
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
                    paste_and_hide(&window, &tree_view, &search_entry);
                }
                Key::Escape => {
                    hide(&window, &search_entry);
                }
                _ => (),
            }
        });

        controller.connect_key_pressed({
            let tree_view = tree_view.clone();

            move |_, key, _, _| {
                fn move_selection(tree_view: &gtk::TreeView, up: bool) {
                    let selection = tree_view.selection();
                    match selection.selected() {
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

                match key {
                    Key::Up => {
                        move_selection(&tree_view, true);
                        Inhibit(true)
                    }
                    Key::Down => {
                        move_selection(&tree_view, false);
                        Inhibit(true)
                    }
                    _ => Inhibit(false),
                }
            }
        });

        controller
    };

    window.add_controller(&key_press_controller);

    window.show()
}

fn launch_application() {
    let application = gtk::Application::new(None, Default::default());

    application.connect_activate(|app| {
        Config::create_file_if_nonexistent();

        let config: Config = Config::from_config_file();

        build_ui(app, config);
    });

    application.run();
}

fn main() {
    use psutil::process::Signal;

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
 * github workflow
 */
