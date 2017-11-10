# Stolen from:
# https://github.com/mmstick/tv-renamer/blob/3f9e274e1b2300209172d4b3c991e9e7952b7259/Makefile

DESTDIR = /usr

all:
	cargo build --release

install:
	install -Dm 755 "target/release/hammond-gtk" "${DESTDIR}/bin/hammond"
	ln -sf "${DESTDIR}/bin/hammond" "${DESTDIR}/bin/hammond-gtk"
	install -Dm 644 "assets/hammond.desktop" "${DESTDIR}/share/applications/hammond.desktop"
	install -Dm 644 README.md "${DESTDIR}/share/doc/hammond/README"
	install -Dm 644 LICENSE "${DESTDIR}/share/licenses/hammond/COPYING"

tar:
	install -Dm 755 "target/release/hammond-gtk" "hammond/bin/hammond"
	ln -sf "hammond/bin/hammond" "hammond/bin/hammond-gtk"
	install -Dm 644 "assets/hammond.desktop" "hammond/share/applications/hammond.desktop"
	install -Dm 644 README.md "hammond/share/doc/hammond/README"
	install -Dm 644 LICENSE "hammond/share/licenses/hammond/COPYING"
	tar cf - "hammond" | xz -zf > "hammond_$(shell uname -m).tar.xz"
	rm -rf hammond

uninstall:
	rm "${DESTDIR}/bin/hammond"
	rm "${DESTDIR}/bin/hammond-gtk"
	rm "${DESTDIR}/share/applications/hammond.desktop"
	rm "${DESTDIR}/share/doc/hammond/README"
	rm "${DESTDIR}/share/licenses/hammond/COPYING"

clean:
	cargo clean
