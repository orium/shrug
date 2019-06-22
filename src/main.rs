/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]

use std::env::args;

use gio::prelude::*;
use gtk::*;

fn build_ui(application: &gtk::Application) {
    let window: ApplicationWindow = gtk::ApplicationWindow::new(application);

    window.set_title("Shrug");
    // WIP! window.type_hint(WindowTypeHint::Dialog);
    window.set_border_width(10);
    window.set_position(WindowPosition::Mouse);
    window.set_default_size(350, 70);
    window.set_keep_above(true);

    let button = gtk::Button::new_with_label("Click me!");

    window.add(&button);

    window.show_all();
}

fn main() {
    let application: Application =
        gtk::Application::new(None, Default::default()).expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}
