## Podcasts release process

- Ensure that the version in meson.build is correct
- Update CHANGELOG.md
- Edit metainfo.xml with the correct version and release notes
- Commit and tag in git
  - In the tag message add the same release notes as in CHANGELOG.md

```
git tag -a '0.4.9'
git push --atomic origin main 0.4.9
```
- Do a post-version release bump
- Open an MR in gitlab and once merged, push the tag

- Open a PR at [Flathub](https://github.com/flathub/org.gnome.Podcasts) with the new tarball from the gitlab release


### Optional maintenance thingies

- Update flatpak modules
- Run `cargo update`, build and commit the new lockfile.
- Check for [outdated](https://github.com/kbknapp/cargo-outdated) crates `cargo install cargo-outdate && cargo outdated -d 1`
