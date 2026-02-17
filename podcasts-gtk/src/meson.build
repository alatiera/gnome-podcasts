global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('VERSION', podcasts_version + version_suffix)
global_conf.set_quoted('LOCALEDIR', podcasts_localedir)
# include_bytes! only takes a string literal
global_conf.set_quoted('RESOURCEFILE', podcasts_resources.full_path())
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

cargo_build_options = [ 
  '--manifest-path', meson.project_source_root() / 'Cargo.toml',
  cargo_release_option
]

cargo_release = custom_target('cargo-build',
                        build_by_default: true,
                        build_always_stale: true,
                        output: ['gnome-podcasts'],
                        install: true,
                        install_dir: podcasts_bindir,
                        console: true,
                        depends: podcasts_resources,
                        env: cargo_env,
                        command: [cargo, 'build',
                                  cargo_build_options,
                                  '&&',
                                  'cp', cargo_target / rust_target / 'podcasts-gtk', '@OUTPUT@',
                        ])
