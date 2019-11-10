/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

pub struct TextClipboard {
    clipboards: [gtk::Clipboard; 3],
}

impl TextClipboard {
    pub fn new() -> TextClipboard {
        TextClipboard {
            clipboards: [
                gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD),
                gtk::Clipboard::get(&gdk::SELECTION_PRIMARY),
                gtk::Clipboard::get(&gdk::SELECTION_SECONDARY),
            ],
        }
    }

    pub fn set(&self, string: &str) {
        for clipboard in self.clipboards.iter() {
            clipboard.set_text(string);
            clipboard.store();
        }
    }
}
