/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub struct TextClipboard {
    clipboard: gtk::Clipboard,
}

impl TextClipboard {
    pub fn new() -> Option<TextClipboard> {
        let display = gdk::Display::get_default()?;
        let clipboard = gtk::Clipboard::get_default(&display)?;

        Some(TextClipboard { clipboard })
    }

    pub fn set(&self, string: &str) {
        self.clipboard.set_text(string);
    }
}
