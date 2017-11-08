use gio::{resources_register, Error, Resource};
use glib::Bytes;

pub fn init() -> Result<(), Error> {
    // load the gresource binary at build time and include/link it into the final binary.
    let res_bytes = include_bytes!("../resources/resources.gresource");

    // Create Resource it will live as long the value lives.
    // TODO: change it into Bytes::From_static once the fix lands
    // https://bugzilla.gnome.org/show_bug.cgi?id=790030
    let gbytes = Bytes::from(&res_bytes.as_ref());
    let resource = Resource::new_from_data(&gbytes)?;
    // let resource = Resource::new_from_data(&res_bytes.as_ref().into())?;

    // Register the resource so It wont be dropped and will continue to live in memory.
    resources_register(&resource);

    Ok(())
}
