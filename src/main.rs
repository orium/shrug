/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]

mod text_clipboard;
mod config;

use gtk::prelude::*;
use std::env::args;
use gio::prelude::*;
use config::Config;
use sublime_fuzzy::FuzzySearch;
use std::time::Instant;
use gtk::{SortType, SortColumn};
use rayon::prelude::*;
use crate::config::Alias;

// WIP! review and organize.
fn build_ui(app: &gtk::Application, config: Config) {
    let glade_src = include_str!("window_main.glade");

    let builder = gtk::Builder::new_from_string(glade_src);

    let window: gtk::Window = builder.get_object("window_main").unwrap();

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
        column.set_sort_column_id(2); // WIP! SortType::Descending
        tree_view.append_column(&column);
    }

    let store: gtk::TreeStore = gtk::TreeStore::new(&[gtk::Type::String, gtk::Type::String, gtk::Type::I64]);

    for alias in config.aliases() {
        store.set(&store.append(None), &[0, 1, 2], &[&alias.key, &alias.value, &0]);
    }

    let sorted_store: gtk::TreeModelSort = gtk::TreeModelSort::new(&store);

    sorted_store.set_sort_column_id(SortColumn::Index(2), SortType::Descending);

    tree_view.set_model(Some(&sorted_store));

    let search_entry: gtk::SearchEntry = builder.get_object("search_entry").unwrap();

    // WIP! can we do this on every change?
    search_entry.connect_search_changed(move |entry| {
        if let Some(text) = entry.get_text().map(|s| s.to_string()) {
            let start = Instant::now();

            let scored_aliases: Vec<(Alias, i64)> = config.aliases().par_bridge().map(|alias| {
                let score: i64 =
                    FuzzySearch::new(&text, alias.key, false).best_match()
                        .map(|m| m.score() as i64)
                        .unwrap_or(0);

                (alias, score)
            }).collect();

            // TODO Do not clear each time.
            store.clear();

            scored_aliases.iter().for_each(|(alias, score)| {
                store.set(&store.append(None), &[0, 1, 2], &[&alias.key, &alias.value, &score]);
            });

            let end = Instant::now();

            println!("ran in {:?}", end - start);
        };
    });

    search_entry.connect_activate(|x| {
        println!("boom {:?}", x);
    });

    window.show_all();
}

fn main() {
    let application = gtk::Application::new(None, Default::default())
        .expect("Failed initializing gtk application");

    application.connect_activate(|app| {
        Config::create_file_if_nonexistent();

        let config: Config = Config::from_config_file();

        build_ui(app, config);
    });

    application.run(&args().collect::<Vec<_>>());
}

/* WIP!
 *
 * Organize code
 * Proper error handling in config
 * Proper error handling in clipboard?
 * README
 */
