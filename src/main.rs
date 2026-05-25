//! Shrug is a small program where you can have a library of named strings. You can then search for
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
//! Note that shrug keeps running in the background after being launched. This is because in X.org,
//! the clipboard content belongs to the program the content originated from. If the program
//! terminates the content of the clipboard gets cleared. (An alternative would be to use a
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
use gtk::prelude::*;
use gtk4 as gtk;
use rayon::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sublime_fuzzy::FuzzySearch;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("runtime directory not available")]
    NoRuntimeDir,
    #[error("socket error: {0}")]
    Socket(std::io::Error),
    #[error("UI widget '{0}' not found")]
    MissingWidget(&'static str),
    #[error("no default display available")]
    NoDisplay,
    #[error("tree view has no model")]
    NoTreeViewModel,
    #[error("failed to read string value from model")]
    ModelStringRead,
}

fn paste_and_hide(
    window: &gtk::Window,
    tree_view: &gtk::TreeView,
    sorted_store: &gtk::TreeModelSort,
    search_entry: &gtk::SearchEntry,
) -> Result<(), Error> {
    if let Some((_, row)) = tree_view.selection().selected() {
        let model = tree_view.model().ok_or(Error::NoTreeViewModel)?;
        let str: String = model.get_value(&row, 1).get().map_err(|_| Error::ModelStringRead)?;
        let display = gdk::Display::default().ok_or(Error::NoDisplay)?;
        TextClipboard::new(&display).set(&str);
    }

    hide(window, search_entry, tree_view, sorted_store);
    Ok(())
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

fn setup_tree_columns(tree_view: &gtk::TreeView) {
    {
        let renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        column.pack_start(&renderer, true);
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
        column.set_clickable(true);
        column.set_sort_indicator(true);
        column.set_sort_column_id(2);
        tree_view.append_column(&column);
    }
}

fn create_store(config: &Config) -> gtk::TreeStore {
    let store = gtk::TreeStore::new(&[Type::STRING, Type::STRING, Type::I64]);

    for alias in config.aliases() {
        store.set(&store.append(None), &[(0, &alias.key), (1, &alias.value), (2, &0i64)]);
    }

    store
}

fn connect_search_handler(
    search_entry: &gtk::SearchEntry,
    tree_view: &gtk::TreeView,
    sorted_store: &gtk::TreeModelSort,
    store: gtk::TreeStore,
    config: Config,
) {
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
}

fn spawn_show_listener(show_listener: Arc<UnixListener>, window: &gtk::Window) {
    let (sender, receiver) = async_channel::bounded(1);

    std::thread::spawn(move || {
        for _ in show_listener.incoming() {
            if sender.send_blocking(()).is_err() {
                break;
            }
        }
    });

    glib::spawn_future_local({
        let window = window.clone();

        async move {
            loop {
                if receiver.recv().await.is_ok() {
                    window.show();
                }
            }
        }
    });
}

fn create_key_controller(
    window: &gtk::Window,
    tree_view: &gtk::TreeView,
    sorted_store: &gtk::TreeModelSort,
    search_entry: &gtk::SearchEntry,
) -> gtk::EventControllerKey {
    let controller = gtk::EventControllerKey::new();

    // For whatever reason these keys can only be observed in the "release_key" event.
    controller.connect_key_released({
        let window = window.clone();
        let tree_view = tree_view.clone();
        let sorted_store = sorted_store.clone();
        let search_entry = search_entry.clone();

        move |_, key, _, _| match key {
            Key::Return => {
                if let Err(e) = paste_and_hide(&window, &tree_view, &sorted_store, &search_entry) {
                    eprintln!("error: {e}");
                }
            }
            Key::Escape => {
                hide(&window, &search_entry, &tree_view, &sorted_store);
            }
            _ => (),
        }
    });

    controller.connect_key_pressed({
        let tree_view = tree_view.clone();

        move |_, key, _, _| match key {
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
}

fn build_ui(
    app: &gtk::Application,
    config: Config,
    show_listener: Arc<UnixListener>,
) -> Result<(), Error> {
    let builder = gtk::Builder::from_string(include_str!("window_main.ui"));

    let window: gtk::Window =
        builder.object("window_main").ok_or(Error::MissingWidget("window_main"))?;
    window.set_application(Some(app));

    let tree_view: gtk::TreeView =
        builder.object("tree_view").ok_or(Error::MissingWidget("tree_view"))?;
    setup_tree_columns(&tree_view);

    let store = create_store(&config);
    let sorted_store = gtk::TreeModelSort::with_model(&store);
    sorted_store.set_sort_column_id(gtk::SortColumn::Index(2), gtk::SortType::Descending);
    tree_view.set_model(Some(&sorted_store));

    if let Some(first_row) = sorted_store.iter_first() {
        tree_view.selection().select_iter(&first_row);
    }

    let search_entry: gtk::SearchEntry =
        builder.object("search_entry").ok_or(Error::MissingWidget("search_entry"))?;
    connect_search_handler(&search_entry, &tree_view, &sorted_store, store, config);

    spawn_show_listener(show_listener, &window);

    let key_controller = create_key_controller(&window, &tree_view, &sorted_store, &search_entry);
    window.add_controller(key_controller);

    window.show();
    Ok(())
}

fn on_activate(app: &gtk::Application, show_listener: &Arc<UnixListener>) -> Result<(), Error> {
    Config::create_file_if_nonexistent()?;
    let config = Config::from_config_file()?;
    build_ui(app, config, Arc::clone(show_listener))
}

fn launch_application(show_listener: UnixListener) {
    let application = gtk::Application::new(None::<&str>, gio::ApplicationFlags::default());
    let show_listener: Arc<UnixListener> = Arc::new(show_listener);

    application.connect_activate(move |app| {
        if let Err(e) = on_activate(app, &show_listener) {
            eprintln!("error: {e}");
            app.quit();
        }
    });

    application.run();
}

enum SendShowSignalOrListen {
    Listener(UnixListener),
    SignalSent,
}

fn send_show_signal_or_listen(
    socket_path: impl AsRef<Path>,
) -> Result<SendShowSignalOrListen, Error> {
    use std::io::ErrorKind;

    match UnixListener::bind(&socket_path) {
        Ok(listener) => Ok(SendShowSignalOrListen::Listener(listener)),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            // We don't know if there's another shrug listening there or not. Let's find out.
            match UnixStream::connect(&socket_path) {
                Ok(_) => Ok(SendShowSignalOrListen::SignalSent),
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    // There's no one there, we should be able to bind now.
                    std::fs::remove_file(&socket_path).map_err(Error::Socket)?;
                    UnixListener::bind(&socket_path)
                        .map(SendShowSignalOrListen::Listener)
                        .map_err(Error::Socket)
                }
                Err(e) => Err(Error::Socket(e)),
            }
        }
        Err(e) => Err(Error::Socket(e)),
    }
}

fn unix_socket_show_signal_path() -> Result<PathBuf, Error> {
    Ok(dirs::runtime_dir().ok_or(Error::NoRuntimeDir)?.join("shrug_show.sock"))
}

fn run() -> Result<(), Error> {
    match send_show_signal_or_listen(unix_socket_show_signal_path()?)? {
        SendShowSignalOrListen::Listener(listener) => {
            launch_application(listener);
        }
        SendShowSignalOrListen::SignalSent => {
            println!("sent signal to running shrug.");
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
    }
}

/* TODO
 *
 * config deamonize
 */
