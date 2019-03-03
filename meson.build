project(
  'gnome-podcasts', 'rust',
  version: '0.4.6',
  license: 'GPLv3',
)

# FIXME: look for cargo and optionally cargo-vendor
# dependency('cargo')

dependency('sqlite3', version: '>= 3.20')
dependency('openssl', version: '>= 1.0')
dependency('dbus-1')

dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('gdk-pixbuf-2.0')
dependency('gtk+-3.0', version: '>= 3.22')
dependency('libhandy-0.0', version: '>= 0.0.8')

dependency('gstreamer-1.0', version: '>= 1.12')
dependency('gstreamer-base-1.0', version: '>= 1.12')
dependency('gstreamer-audio-1.0', version: '>= 1.12')
dependency('gstreamer-video-1.0', version: '>= 1.12')
dependency('gstreamer-player-1.0', version: '>= 1.12')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.12')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.12')
dependency('gstreamer-bad-audio-1.0', version: '>= 1.12')

cargo = find_program('cargo', required: true)
gresource = find_program('glib-compile-resources', required: true)
gschemas = find_program('glib-compile-schemas', required: true)
cargo_vendor = find_program('cargo-vendor', required: false)

if get_option('profile') == 'development'
  profile = 'Devel'
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

application_id = 'org.gnome.Podcasts@0@'.format(profile)
i18n = import('i18n')

subdir('podcasts-gtk/po')
podir = join_paths (meson.source_root (), 'podcasts-gtk', 'po')

podcasts_version = meson.project_version()

podcasts_prefix = get_option('prefix')
podcasts_bindir = join_paths(podcasts_prefix, get_option('bindir'))
podcasts_localedir = join_paths(podcasts_prefix, get_option('localedir'))

podcasts_conf = configuration_data()
podcasts_conf.set('appid', application_id)
podcasts_conf.set('bindir', podcasts_bindir)

datadir = get_option('datadir')
subdir('podcasts-gtk/resources')

cargo_script = find_program('scripts/cargo.sh')

cargo_release = custom_target('cargo-build',
                        build_always_stale: true,
                        output: ['gnome-podcasts'],
                        install: true,
                        install_dir: podcasts_bindir,
                        console: true,
                        command: [cargo_script,
                                  '@CURRENT_SOURCE_DIR@',
                                  '@OUTPUT@',
                                  podcasts_localedir,
                                  application_id,
                                  podcasts_version + version_suffix,
                                  profile
                                 ])

run_target('release', command: ['scripts/release.sh',
                                meson.project_name() + '-' + podcasts_version
                               ])
