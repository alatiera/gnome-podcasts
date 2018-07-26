# Adatped from:
# https://gitlab.gnome.org/danigm/fractal/blob/6e2911f9d2353c99a18a6c19fab7f903c4bbb431/meson.build

project(
  'gnome-podcasts', 'rust',
  version: '0.4.1',
  license: 'GPLv3',
)

podcasts_version = meson.project_version()
version_array = podcasts_version.split('.')
podcasts_major_version = version_array[0].to_int()
podcasts_minor_version = version_array[1].to_int()
podcasts_version_micro = version_array[2].to_int()

podcasts_prefix = get_option('prefix')
podcasts_bindir = join_paths(podcasts_prefix, get_option('bindir'))

podcasts_conf = configuration_data()
podcasts_conf.set('BINDIR', podcasts_bindir)

datadir = get_option('datadir')
icondir = join_paths(datadir, 'icons')
subdir('podcasts-gtk/resources')

cargo = find_program('cargo', required: false)
gresource = find_program('glib-compile-resources', required: false)
cargo_vendor = find_program('cargo-vendor', required: false)
cargo_script = find_program('scripts/cargo.sh')

cargo_release = custom_target('cargo-build',
                        build_by_default: true,
                        build_always: true,
                        output: ['gnome-podcasts'],
                        install: true,
                        install_dir: podcasts_bindir,
                        command: [cargo_script, '@CURRENT_SOURCE_DIR@', '@OUTPUT@'])

run_target('release', command: ['scripts/release.sh',
                                meson.project_name() + '-' + podcasts_version
                               ])