[![Build Status](https://github.com/orium/shrug/workflows/CI/badge.svg)](https://github.com/orium/shrug/actions?query=workflow%3ACI)
[![Dependency status](https://deps.rs/repo/github/orium/shrug/status.svg)](https://deps.rs/repo/github/orium/shrug)
[![crates.io](https://img.shields.io/crates/v/shrug.svg)](https://crates.io/crates/shrug)
[![Downloads](https://img.shields.io/crates/d/shrug.svg)](https://crates.io/crates/shrug)
[![Downloads github](https://img.shields.io/github/downloads/orium/shrug/total.svg?label=github%20downloads)](https://github.com/orium/shrug/releases)
[![Github stars](https://img.shields.io/github/stars/orium/shrug.svg?logo=github)](https://github.com/orium/shrug/stargazers)
[![License](https://img.shields.io/crates/l/shrug.svg)](./LICENSE.md)

# ¯\\\_(ツ)\_/¯

<!-- cargo-rdme start -->

Shrug is a small program where you can have a library of named strings. You can then search for
those strings to have them readily available in your clipboard.

This is what it looks like:

<p align="center">
<img src="https://raw.githubusercontent.com/orium/shrug/main/images/shrug.png" width="300">
</p>

I suggest you add a key binding in your window manager to launch shrug.

Note that shrug keeps running in the background after being launched. This is because in X.org,
the clipboard content belongs to the program the content originated from. If the program
terminates the content of the clipboard gets cleared. (An alternative would be to use a
clipboard manager ¯\\\_(ツ)\_/¯.)

<!-- cargo-rdme end -->
