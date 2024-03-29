subdir('icons')

desktop_conf = configuration_data()
desktop_conf.set('icon', application_id)
desktop_file = i18n.merge_file (
  type: 'desktop',
  input: configure_file(
    input: files('org.gnome.Podcasts.desktop.in.in'),
    output: 'org.gnome.Podcasts.desktop.in',
    configuration: desktop_conf
  ),
  output: '@0@.desktop'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'applications')
)
desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test(
    'validate-desktop',
    desktop_file_validate,
    args: [
      desktop_file.full_path()
    ]
  )
endif

appdata_conf = configuration_data()
appdata_conf.set('appid', application_id)
appdata_file = i18n.merge_file (
  input: configure_file(
    input: files('org.gnome.Podcasts.appdata.xml.in.in'),
    output: 'org.gnome.Podcasts.appdata.xml.in',
    configuration: appdata_conf
  ),
  output: '@0@.appdata.xml'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'metainfo')
)
appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test(
    'validate-appdata', appstreamcli,
    args: [
      'validate', '--no-net', '--explain', appdata_file.full_path()
    ]
  )
endif

configure_file(
  input: 'org.gnome.Podcasts.gschema.xml.in',
  output: 'org.gnome.Podcasts.gschema.xml',
  configuration: podcasts_conf,
  install: true,
  install_dir: join_paths(datadir, 'glib-2.0', 'schemas')
)

configure_file(
  input: 'org.gnome.Podcasts.service.in',
  output: '@0@.service'.format(application_id),
  configuration: podcasts_conf,
  install_dir: join_paths(datadir,'dbus-1', 'services')
)

podcasts_resources = gnome.compile_resources(
  'resources',
  'resources.xml',
  gresource_bundle: true,
  source_dir: meson.current_build_dir()
)

# Validating schemas
test('Validate schema file', gschemas,
  args: ['--strict', '--dry-run', meson.current_source_dir()]
)
