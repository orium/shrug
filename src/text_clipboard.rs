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
