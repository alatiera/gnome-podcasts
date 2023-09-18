## Podcasts release proccess

- Ensure that the version in meson.build is correct
- Update CHANGELOG.md
- Edit appdata.xml with the correct version and release notes
- Commit and tag in git

```
git tag -a '0.4.9' -m '0.4.9'
git push --atomic origin master 0.4.9
```

- Make a tarball for flathub
  - Open a Build Terminal in Builder. Shift+Control+Alt+T
  - Run the following commands
```
source /usr/lib/sdk/rust-stable/enable.sh
meson dist --no-tests
```
  - Copy the created tar.xz and sha256sum to ~/Downloads

- Make a release on GitLab
  - Add the same release notes as in CHANGELOG.md
  - At the very end, add tar.xz and sha256sum file 
- Open a PR at https://github.com/flathub/org.gnome.Podcasts
- Post-release version bump meson.build



### Optional maintenance thingies

- Update flatpak modules
- Run `cargo update`, build and commit the new lockfile.
- Check for [outdated](https://github.com/kbknapp/cargo-outdated) crates `cargo install cargo-outdate && cargo outdated -d 1`

