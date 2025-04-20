project(
  'gnome-podcasts', 'rust',
  version: '25.2',
  license: 'GPLv3',
  meson_version: '>= 1.4',
)

dependency('sqlite3', version: '>= 3.20')
dependency('openssl', version: '>= 1.0')
dependency('dbus-1')

dependency('glib-2.0', version: '>= 2.76')
dependency('gio-2.0', version: '>= 2.76')
dependency('gdk-pixbuf-2.0')
dependency('gtk4', version: '>= 4.15.3')
dependency('libadwaita-1', version :'>=1.6')

dependency('gstreamer-1.0', version: '>= 1.22')
dependency('gstreamer-base-1.0', version: '>= 1.22')
dependency('gstreamer-audio-1.0', version: '>= 1.22')
dependency('gstreamer-play-1.0', version: '>= 1.22')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.22')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.22')
dependency('gstreamer-bad-audio-1.0', version: '>= 1.22')

cargo = find_program('cargo', required: true)
gresource = find_program('glib-compile-resources', required: true)
gschemas = find_program('glib-compile-schemas', required: true)

if get_option('profile') == 'development'
  profile = '.Devel'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD').stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format (vcs_tag)
  endif
else
  profile = ''
  version_suffix = ''
endif

podcast_toml = files(
  'Cargo.toml',
  'Cargo.lock',
  'podcasts-data/Cargo.toml',
  'podcasts-gtk/Cargo.toml',
)

application_id = 'org.gnome.Podcasts@0@'.format(profile)
i18n = import('i18n')
gnome = import('gnome')

subdir('podcasts-gtk/po')
podir = join_paths (meson.project_source_root (), 'podcasts-gtk', 'po')

podcasts_version = meson.project_version()

podcasts_prefix = get_option('prefix')
podcasts_bindir = join_paths(podcasts_prefix, get_option('bindir'))
podcasts_localedir = join_paths(podcasts_prefix, get_option('localedir'))

podcasts_conf = configuration_data()
podcasts_conf.set('appid', application_id)
podcasts_conf.set('bindir', podcasts_bindir)

datadir = get_option('datadir')
subdir('podcasts-gtk/resources')

test_script = find_program('scripts/test.sh')

subdir('podcasts-gtk/src')

meson.add_dist_script(
  'scripts/dist-vendor.sh',
  meson.project_build_root() / 'meson-dist' / meson.project_name() + '-' + podcasts_version,
  meson.project_source_root()
)

test(
  'cargo-test',
  test_script,
  args: meson.project_build_root(),
  workdir: meson.project_source_root(),
  timeout: 3000
)

gnome.post_install(
  gtk_update_icon_cache: true,
  glib_compile_schemas: true,
  update_desktop_database: true,
)
