global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('VERSION', podcasts_version + version_suffix)
global_conf.set_quoted('LOCALEDIR', podcasts_localedir)
global_conf.set_quoted('PKGDATADIR', pkgdatadir)
config_rs = configure_file(
  input: 'config.rs.in',
  output: 'config.rs',
  configuration: global_conf
)

run_command(
  'cp',
  config_rs,
  meson.current_source_dir(),
  check: true
)

podcasts_sources = files(
  'stacks/content.rs',
  'stacks/home.rs',
  'stacks/mod.rs',
  'stacks/populated.rs',
  'stacks/show.rs',
  'widgets/aboutdialog.rs',
  'widgets/base_view.rs',
  'widgets/empty.rs',
  'widgets/empty_show.rs',
  'widgets/episode.rs',
  'widgets/episode_menu.rs',
  'widgets/episode_description.rs',
  'widgets/home_view.rs',
  'widgets/mod.rs',
  'widgets/player.rs',
  'widgets/show.rs',
  'widgets/show_menu.rs',
  'widgets/shows_view.rs',
  'widgets/short_desc.rs',
  'widgets/short_desc_layout.rs',
  'app.rs',
  'episode_description_parser.rs',
  'headerbar.rs',
  'i18n.rs',
  'main.rs',
  'manager.rs',
  'settings.rs',
  'utils.rs',
  'window.rs',
)

cargo_release = custom_target('cargo-build',
                        build_by_default: true,
                        input: [
                                podcast_toml,
                                data_sources,
                                podcasts_sources,
                               ],
                        output: ['gnome-podcasts'],
                        install: true,
                        install_dir: podcasts_bindir,
                        console: true,
                        depends: podcasts_resources,
                        command: [cargo_script,
                                  meson.build_root(),
                                  meson.source_root(),
                                  '@OUTPUT@',
                                  profile,
                        ])
