project(
    'ArcamClient',
    'rust',
    version: '0.0.0', # Ensure this is the same as the version number in Cargo.toml
    default_options: ['buildtype=release'],
    license: 'GPLv3',
)

application_id = 'uk.org.winder.arcamclient'

prefix = get_option('prefix')
bindir = get_option('bindir')
datadir = get_option('datadir')

dependency('glib-2.0', version: '>= 2.56') # 2.60.6 on flatpak
dependency('gio-2.0', version: '>= 2.56') # 2.60.6 on flatpak
dependency('gtk+-3.0', version: '>= 3.22') # 3.24.20 on flatpak

cargo = find_program('cargo')
cargo_script = find_program('scripts/cargo.sh')

sources = run_command('sh', '-c', 'cd ${MESON_SOURCE_ROOT} && ls src/*.rs').stdout().strip().split()

arcamclient_target_name = 'arcamclient'

arcamclient = custom_target(
    arcamclient_target_name,
    build_by_default: true,
    console: true,
    input: sources,
    output: [arcamclient_target_name],
    install: true,
    install_dir: bindir,
    command: [cargo_script, '@CURRENT_SOURCE_DIR@', '@OUTPUT@', get_option('buildtype'),]
)

install_man('doc/arcamclient.1')

install_data(
    'data/@0@.desktop'.format(application_id),
    install_dir: datadir / 'applications'
)

install_data(
    'data/@0@.metainfo.xml'.format(application_id),
    install_dir: datadir / 'metainfo'
)

install_data(
    'src/resources/@0@.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

meson.add_dist_script(
  'scripts/dist-vendor.sh',
  meson.build_root() / 'meson-dist' / meson.project_name() + '-' + meson.project_version(),
  meson.source_root()
)
