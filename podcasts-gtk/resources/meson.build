subdir('icons')

i18n.merge_file ('desktop-file',
  type: 'desktop',
  input: 'org.gnome.Podcasts.desktop.in',
  output: 'org.gnome.Podcasts.desktop',
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'applications')
)

i18n.merge_file ('appdata-file',
  input: 'org.gnome.Podcasts.appdata.xml.in',
  output: 'org.gnome.Podcasts.appdata.xml',
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'appdata')
)

install_data('org.gnome.Podcasts.gschema.xml', install_dir: join_paths(datadir, 'glib-2.0', 'schemas'))

configure_file(input: 'org.gnome.Podcasts.service.in',
               output: 'org.gnome.Podcasts.service',
               configuration: podcasts_conf,
               install_dir: join_paths([datadir,'dbus-1/services']))

meson.add_install_script('../../scripts/compile-gschema.py')
