// static_resource.rs
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

use gio::{resources_register, Error, Resource};
use glib::Bytes;

pub(crate) fn init() -> Result<(), Error> {
    // load the gresource binary at build time and include/link it into the final
    // binary.
    let res_bytes = include_bytes!("../resources/resources.gresource");

    // Create Resource it will live as long the value lives.
    let gbytes = Bytes::from_static(res_bytes.as_ref());
    let resource = Resource::new_from_data(&gbytes)?;

    // Register the resource so it won't be dropped and will continue to live in
    // memory.
    resources_register(&resource);

    Ok(())
}
