# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added:

### Changed:
- Download Cancel button was changed to an Icon instead of a label !72
- The applciation will no longer scale below 360p in width 1933c79f7a87d8261d91ca4e14eb51c1ddc66624
- Update to the latest HIG 5050dda4d2f75b706842de8507d115dd5a1bd0a9

### Fixed:
- Fixed a regression where indexing feeds was blocking the `tokio reactor` #88 !70
- Episodeds Listbox no longer resizes when a download starts #89 !72
- The `total_size` label of the `EpisodeWidget` now behaves correctly if the request fails. #90 !73
- The Pipeline will no longer log things in stderr for Requests that returned 304 and are expected to be skipped. da361d0cb93cd8edd076859b2c607509a96dac8d

### Removed:

### Translations:

**Added**
- Brazilian Portuguese translation 586cf16f
- Swedish translation 2e527250
- Italian translation a23297e5
- Friulian translation 60e09c0d
- Hungarian translation 2751a828
- Croatian translation 0476b67b
- Latvian translation a681b2c9
- Czech translation 3563a964
- Catalan translation 6ea3fc91

**Updated**
- German translation
- Finnish translation
- Polish translation
- Turkish translation
- Croatian translation 
- Indonesian translation
- Spanish translation 


## [0.4.5] - 2018-08-31

### Added:
- [OARS](https://hughsie.github.io/oars/) Tags where added for compatibility with Store clients b0c94dd9
- Daniel added support for Translations !46
- Svitozar Cherepii(@svito) created a [wiki page](https://wiki.gnome.org/Apps/Podcasts) 70e79e50
- Libhandy was added as a dependancy #70
- Development builds can now be installed in parallel with stable builds !64

### Changed:
- The update indication was moved to an In-App notification #72
- The app icon's accent color was changed from orange to red 0dfb4859
- The stack switcher in the Headerbar is now insesitive on Empty Views !63

### Fixed:
- Improved handling of HTTP redirections #64 !61 !62
- Fixed a major performance regression when loading show covers !67
- More refference cycles have been fixed !59
- OPML import dialog now exits properly and no longer keeps the application from shuting down !65
- Update action is disabled if there isn't something to update #71

### Translations:
- Added Finish 93696026
- Added Polish 1bd6efc0
- Added Turkish 73929f2d
- Added Spanish !46
- Added German 6b6c390c
- Added Galician 0060a634
- Added Indonesian ded0224f
- Added Korean 36f16963


## [0.4.4] - 2018-07-31

### Changed:
- `SendCell` crate was replaced with `Fragile`. (Jorda Petridis) 838320785ebbea94e009698b473495cfec076f54
- Update dependancies (Jorda Petridis) 91bea8551998b16e44e5358fdd43c53422bcc6f3

### Fixed:
- Fix more refference cycles. (Jorda Petridis) 3496df24f8d8bfa8c8a53d8f00262d42ee39b41c
- Actually fix cargo-vendor (Jorda Petridis)

## [0.4.3] - 2018-07-27

### Fixed:

- Fix the cargo vendor config for the tarball releash script. (Jorda Petridis) a2440c19e11ca4dcdbcb67cd85259a41fe3754d6

## [0.4.2] - 2018-07-27

### Changed:

- Minimum size requested by the Views. (Jorda Petridis) 7c96152f3f53f271247230dccf1c9cd5947b685f

### Fixed:

- Screenshot metadata in appstream data. (Jorda Petridis) a2440c19e11ca4dcdbcb67cd85259a41fe3754d6

## [0.4.1] - 2018-07-26
### Added:

- Custom icons for the fast-forward and rewind actions in the Player were added. (Tobias Bernard) e77000076b3d78b8625f4c7ef367376d0130ece6
- Hicolor and symbolic icons for the Application. (Tobias Bernard and Sam Hewitt) edae1b04801dba9d91d5d4145db79b287f0eec2c
- Basic prefferences dialog (Zander Brown). [34](https://gitlab.gnome.org/World/podcasts/merge_requests/34)
- Dbus service preperation. Not used till the MPRIS2 integration has landed. (Zander Brown) [42](https://gitlab.gnome.org/World/podcasts/merge_requests/42)
- Episodes and Images will only get drawn when needed. Big Performance impact. (Jordan Petridis) [43](https://gitlab.gnome.org/World/podcasts/merge_requests/43)

### Changed:

- The `ShowWidget` control button were moved to a secondary menu in the Headerbar. (Jordan Petridis) 536805791e336a3e112799be554706bb804d2bef
- EmptyView layout improvements. (Jorda Petridis) 3c3d6c1e7f15b88308a9054b15a6ca0d8fa233ce 518ea9c8b57885c44bda9c418b19fef26ae0e55d
- Improved the `AddButton` behavior. (Jorda Petridis) 67ab54f8203f19aad198dc49e935127d25432b41

### Fixed:

- A couple reffence cycles where fixed. (Jorda Petridis)

### Removed:

- The delay between the application startup and the `update_on_startup` action. (Jorda Petridis) 7569465a612ee5ef84d0e58f4e1010c8d14080d4

## [0.4.0] - 2018-07-04
### Added:
- Keyboard Shortcuts and a Shortcuts dialog were implemented. (ZanderBrown)
[!33](https://gitlab.gnome.org/World/podcasts/merge_requests/33)

### Changed:
- The `FileChooser` of the OPML import was changed to use the `FileChooserNative` widget/API. (ZanderBrown)
[!33](https://gitlab.gnome.org/World/podcasts/merge_requests/33)
- The `EpisdeWidget` was refactored.
[!38](https://gitlab.gnome.org/World/podcasts/merge_requests/38)
- `EpisdeWidget`'s progressbar was changed to be non-blocking and should feel way more responsive now. 9b0ac5b83dadecdff51cd398293afdf0d5276012
- An embeded audio player was implemented!
[!40](https://gitlab.gnome.org/World/podcasts/merge_requests/40)
- Various Database changes.
[!41](https://gitlab.gnome.org/World/podcasts/merge_requests/41)

### Fixed:
- Fixed a bug whre the about dialog would be unclosable. (ZanderBrown) [!37](https://gitlab.gnome.org/World/podcasts/merge_requests/37)

## [0.3.4] - 2018-05-20
### Fixed:
- Flatpak can now access the Home folder. This fixes the OPML import feature from
not being able to access any file.

## [0.3.3] - 2018-05-19
### Added:
- Initial functionality for importing shows from an OPML file was implemented.
- ShowsView now rembmers the vertical alignment of the scrollbar between refreshes. 4d2b64e79d8518454b3677612664cd32044cf837

### Changed:
- Minimum `rustc` version requirment was bumped to `1.26`
- Some animations should be smoother now. 7d598bb1d08b05fd5ab532657acdad967c0afbc3
- InAppNotification now can be used to propagate some erros to the user. 7035fe05c4741b3e7ccce6827f72766226d5fc0a and 118dac5a1ab79c0b4ebe78e88256a4a38b138c04

### Fixed:
- Fixed a of by one bug in the `ShowsView` where the last show was never shown. bd12b09cbc8132fd39a266fd091e24bc6c3c040f

## [0.3.2] - 2018-05-07
### Added:
- Vies now have a new fancy scrolling animation when they are refereshed.

### Changed:
- Downlaoding and loading images now is done asynchronously and is not blocking programs execution.
[#7](https://gitlab.gnome.org/World/podcasts/issues/7)
- Bold, italics links and some other `html` tags can now be rendered in the Show Description.
[#25](https://gitlab.gnome.org/World/podcasts/issues/25)
- `Rayon` Threadpools are now used instead of unlimited one-off threads.
- `EpisdeWidget`s are now loaded asynchronously accross views.
- `EpisodeWidget`s no longer trigger a `View` refresh for trivial stuff 03bd95184808ccab3e0ea0e3713a52ee6b7c9ab4
- `ShowWidget` layout was changed 9a5cc1595d982f3232ee7595b83b6512ac8f6c88
- `ShowWidget` Description is inside a scrolled window now

### Fixed:
- `EpisodeWidget` Height now is consistent accros views [#57](https://gitlab.gnome.org/World/podcasts/issues/57)
- Implemented a tail-recursion loop to follow-up when a feed redirects to another url. c6a24e839a8ba77d09673f299cfc1e64ba7078f3

### Removed:
- Removed the custom configuration file and replaced instructions to just use meson. 1f1d4af8ba7db8f56435d13a1c191ecff3d4a85b

## [0.3.1] - 2018-03-28
### Added:
- Ability to mark all episodes of a Show as watched.
[#47](https://gitlab.gnome.org/World/podcasts/issues/47)
- Now you are able to subscribe to itunes™ podcasts by using the itunes link of the show.
[#49](https://gitlab.gnome.org/World/podcasts/issues/49)
- Hammond now remembers the window size and position. (Rowan Lewis)
[#50](https://gitlab.gnome.org/World/podcasts/issues/50)
- Implemnted the initial work for integrating with GSettings and storing preferences. (Rowan Lewis)
[!22](https://gitlab.gnome.org/World/podcasts/merge_requests/22) [!23](https://gitlab.gnome.org/World/podcasts/merge_requests/23)
- Shows without episodes now display an empty message similar to EmptyView.
[#44](https://gitlab.gnome.org/World/podcasts/issues/44)

### Changed:
- EpisdeWidget has been reimplemented as a compile time state machine.
[!18](https://gitlab.gnome.org/World/podcasts/merge_requests/18)
- Content Views no longer scroll horizontally when shrunk bellow their minimum size.
[#35](https://gitlab.gnome.org/World/podcasts/issues/35)
- Some requests now use the Tor Browser's user agent. (Rowan Lewis)
[#53](https://gitlab.gnome.org/World/podcasts/issues/53)

### Fixed:
- Double border aroun the main window was fixed. (Rowan Lewis)
[#52](https://gitlab.gnome.org/World/podcasts/issues/52)

## [0.3.0] - 2018-02-11
- Tobias Bernard Redesigned the whole Gtk+ client.
- Complete re-write of hammond-data and hammond-gtk modules.
- Error handling for all crates was migrated from error-chain to Failure.
- Hammond-data now uses futures to parse feeds.
- Custom gtk-widgets are now composed structs as opposed to functions returning Gtk widgets.

## [0.2.0] - 2017-11-28
- Database Schema Breaking Changes.
- Added url sanitization. #4.
- Reworked and refactored of the hammond-data API.
- Added some more unit tests
- Documented hammond-data public API.

## [0.1.1] - 2017-11-13
- Added appdata.xml file

## [0.1.0] - 2017-11-13
- Initial Release
