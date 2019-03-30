// lib.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

#![recursion_limit = "1024"]
#![allow(unknown_lints)]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name))]
// Enable lint group collections
#![warn(nonstandard_style, edition_2018, rust_2018_idioms, bad_style, unused)]
// standalone lints
#![warn(
    const_err,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    plugin_as_library,
    unconditional_recursion,
    unions_with_drop_fields,
    while_true,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    elided_lifetime_in_paths,
    missing_copy_implementations
)]
// #![deny(warnings)]

extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

extern crate glob;
extern crate mime_guess;
#[macro_use]
extern crate podcasts_data;
extern crate reqwest;
extern crate tempdir;

pub mod downloader;
pub mod errors;
