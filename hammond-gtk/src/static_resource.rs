extern crate gio_sys;
extern crate glib_sys;
extern crate libc;

use gio::{resources_register, Error, Resource};
use glib::Bytes;

use std;

pub fn init() -> Result<(), Error> {
    // load the gresource binary at build time and include/link it into the final binary.
    let res_bytes = include_bytes!("../resources/resources.gresource");

    // // Create Resource it will live as long the value lives.
    // let resource = Resource::new_from_data(&Bytes::from(&res_bytes))?;
    // let resource = Resource::new_from_data(&res_bytes.into())?;

    // // Register the resource so It wont be dropped and will continue to live in memory.
    // resources_register(&resource);

    unsafe {
        // gbytes and resource will not be freed
        let gbytes =
            glib_sys::g_bytes_new(res_bytes.as_ptr() as *const libc::c_void, res_bytes.len());
        let resource = gio_sys::g_resource_new_from_data(gbytes, std::ptr::null_mut());
        gio_sys::g_resources_register(resource);
    }

    Ok(())
}
