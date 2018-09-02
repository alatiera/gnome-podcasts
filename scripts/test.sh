#! /usr/bin/sh

export MANIFEST_PATH="org.gnome.PodcastsDevel.json"
export RUNTIME_REPO="https://sdk.gnome.org/gnome-nightly.flatpakrepo"
export FLATPAK_MODULE="gnome-podcasts"
export CONFIGURE_ARGS="-Dprofile=development"
export DBUS_ID="org.gnome.PodcastsDevel"
export BUNDLE="org.gnome.Podcasts.Devel.flatpak"

flatpak-builder --stop-at=${FLATPAK_MODULE} --keep-build-dirs --force-clean app ${MANIFEST_PATH}
# https://gitlab.gnome.org/World/podcasts/issues/55
# Force regeneration of gresources regardless of artifacts chage
flatpak-builder --run app ${MANIFEST_PATH} glib-compile-resources --sourcedir=podcasts-gtk/resources/ podcasts-gtk/resources/resources.xml

# Build the flatpak repo
flatpak-builder --run app ${MANIFEST_PATH} meson --prefix=/app ${CONFIGURE_ARGS} build
flatpak-builder --run \
    --env=CARGO_TARGET_DIR="../target_build/" \
    app ${MANIFEST_PATH} \
    ninja -C build install
    # ninja -C build gnome-podcasts-pot; ninja -C _build gnome-podcasts-update-po

# Run the tests
xvfb-run -a -s "-screen 0 1024x768x24" \
    flatpak-builder --run \
    --env=APP_ID="org.gnome.PodcastsDevel" \
    --env=LOCALEDIR="./podcasts-gtk/po" \
    --env=VERSION="0.0.0" \
    --env=CARGO_HOME="target/cargo-home" \
    --env=CARGO_TARGET_DIR="target_test/" \
    app ${MANIFEST_PATH} \
    cargo test -j 1 -- --test-threads=1

# Create a flatpak bundle
# flatpak-builder --finish-only app ${MANIFEST_PATH}
# flatpak build-export repo app
# flatpak build-bundle repo ${BUNDLE} --runtime-repo=${RUNTIME_REPO} ${DBUS_ID}