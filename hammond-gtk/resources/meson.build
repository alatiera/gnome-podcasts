subdir('icons')

install_data('org.gnome.Hammond.desktop', install_dir : datadir + '/applications')
install_data('org.gnome.Hammond.appdata.xml', install_dir : datadir + '/appdata')
install_data('org.gnome.Hammond.gschema.xml', install_dir: join_paths(datadir, 'glib-2.0', 'schemas'))


configure_file(input: 'org.gnome.Hammond.service.in',
               output: 'org.gnome.Hammond.service',
               configuration: hammond_conf,
               install_dir: join_paths([datadir,'dbus-1/services']))

meson.add_install_script('../../scripts/compile-gschema.py')
