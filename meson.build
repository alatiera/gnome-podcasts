# Adatped from:
# https://gitlab.gnome.org/danigm/fractal/blob/6e2911f9d2353c99a18a6c19fab7f903c4bbb431/meson.build

project(
  'gnome-podcasts', 'rust',
  version: '0.4.5',
  license: 'GPLv3',
)

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
version_array = podcasts_version.split('.')
podcasts_major_version = version_array[0].to_int()
podcasts_minor_version = version_array[1].to_int()
podcasts_version_micro = version_array[2].to_int()

podcasts_prefix = get_option('prefix')
podcasts_bindir = join_paths(podcasts_prefix, get_option('bindir'))
podcasts_localedir = join_paths(podcasts_prefix, get_option('localedir'))

podcasts_conf = configuration_data()
podcasts_conf.set('appid', application_id)
podcasts_conf.set('bindir', podcasts_bindir)

datadir = get_option('datadir')
subdir('podcasts-gtk/resources')

cargo = find_program('cargo', required: false)
gresource = find_program('glib-compile-resources', required: false)
cargo_vendor = find_program('cargo-vendor', required: false)
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
