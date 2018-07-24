subdir('icons')

install_data('org.gnome.Podcasts.desktop', install_dir : datadir + '/applications')
install_data('org.gnome.Podcasts.appdata.xml', install_dir : datadir + '/appdata')
install_data('org.gnome.Podcasts.gschema.xml', install_dir: join_paths(datadir, 'glib-2.0', 'schemas'))


configure_file(input: 'org.gnome.Podcasts.service.in',
               output: 'org.gnome.Podcasts.service',
               configuration: podcasts_conf,
               install_dir: join_paths([datadir,'dbus-1/services']))

meson.add_install_script('../../scripts/compile-gschema.py')
