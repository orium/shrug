/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use gdk4 as gdk;

use gdk::Clipboard;
use gdk4::prelude::*;

pub struct TextClipboard {
    clipboards: [Clipboard; 2],
}

impl TextClipboard {
    pub fn new(display: &gdk::Display) -> TextClipboard {
        TextClipboard { clipboards: [display.clipboard(), display.primary_clipboard()] }
    }

    pub fn set(&self, string: &str) {
        for clipboard in &self.clipboards {
            clipboard.set_text(string);
        }
    }
}
