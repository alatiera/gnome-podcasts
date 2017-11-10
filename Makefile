# Stolen from:
# https://github.com/mmstick/tv-renamer/blob/3f9e274e1b2300209172d4b3c991e9e7952b7259/Makefile

DESTDIR = /usr
version = $(shell awk 'NR == 3 {print substr($$3, 2, length($$3)-2)}' Cargo.toml)

all:
	cargo build --release

install:
	install -Dm 755 "target/release/hammond-gtk" "${DESTDIR}/bin/hammond"
	ln -sf "${DESTDIR}/bin/hammond" "${DESTDIR}/bin/hammond-gtk"
	install -Dm 644 "assets/hammond.desktop" "${DESTDIR}/share/applications/hammond.desktop"
	install -Dm 644 README.md "${DESTDIR}/share/doc/hammond/README"
	install -Dm 644 LICENSE "${DESTDIR}/share/licenses/hammond/COPYING"

uninstall:
	rm "${DESTDIR}/bin/hammond"
	rm "${DESTDIR}/bin/hammond-gtk"
	rm "${DESTDIR}/share/applications/hammond.desktop"
	rm "${DESTDIR}/share/doc/hammond/README"
	rm "${DESTDIR}/share/licenses/hammond/COPYING"

clean:
	cargo clean
