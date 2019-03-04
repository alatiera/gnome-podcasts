global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('VERSION', podcasts_version + version_suffix)
global_conf.set_quoted('LOCALEDIR', podcasts_localedir)
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

cargo_release = custom_target('cargo-build',
                        build_always_stale: true,
                        output: ['gnome-podcasts'],
                        install: true,
                        install_dir: podcasts_bindir,
                        console: true,
                        command: [cargo_script,
                                  '@SOURCE_ROOT@',
                                  '@OUTPUT@',
                                  podcasts_localedir,
                                  application_id,
                                  podcasts_version + version_suffix,
                                  profile
                                 ])
