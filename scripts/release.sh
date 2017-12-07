#!/bin/sh

VERSION=$1
DEST=${MESON_BUILD_ROOT}
DIST=$DEST/dist/$VERSION


cd "${MESON_SOURCE_ROOT}"
mkdir -p $DIST

# copying files
cp -rf hammond-data $DIST
cp -rf hammond-gtk $DIST
cp -rf hammond-downloader $DIST
cp Cargo.toml $DIST
cp configure $DIST
cp meson.build $DIST
cp Hammond.doap $DIST
cp LICENSE $DIST
cp README.md $DIST
cp -rf assets $DIST
cp -rf scripts $DIST

# cargo vendor
cargo vendor
mkdir $DIST/.cargo
cat <<EOF > $DIST/.cargo/config
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF
cp -rf vendor $DIST/

# packaging
cd $DEST/dist
tar -czvf $VERSION.tar.gz $VERSION

