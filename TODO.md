## TODOs:

**General:**

- [ ] Add CONTRIBUTING.md
- [ ] Write docs


## Priorities:

**Would be nice:**

- [x] Possibly Convert description labels to textview.
- [ ] Make Podcast cover fetchng and loading not block the execution of the program at startup.
- [ ] Re-design EpisodeWidget.
- [ ] Lazy evaluate episode loading based on the podcast_widget's view scrolling.
- [ ] Headerbar back button and stack switching


**Unhack stuff:**

- [ ] Url sanitization
- [ ] Fix downloader .ext parsing


**FIXME:**

- [ ] Fix Etag/Last-modified implementation


**Look into:**

- [ ] Icons && install stuff && flatpak

* Neither flatpak nor meson support atm building from cargo.


**DB changes:**

- [x] episodes: add watched field
- [x] Podcast deletion
- [x] Download cleaner
- [ ] Discuss and decide when to schedule the download cleaner.
- [ ] Mark episodes/podcast for archival
- [ ] Mark stuff as Favorite. Maybe auto-archive favorites?
- [ ] New episode notifier on podcast_flowbox_child, like the one vocal has


## Secondary:

- [ ] Unplayed Only and Downloaded only view.
- [ ] Auto-updater, update on startup
- [ ] Make use of file metadas, [This](https://github.com/GuillaumeGomez/audio-video-metadata) might be helpfull.
- [ ] Notifications
- [ ] Episode queue
- [ ] Embedded player
- [ ] MPRIS integration
- [ ] Search Implementation
- [ ] OPML import/export // Probably need to create a crate.


## Third: 

- [ ] soundcloud and itunes feeds // [This](http://getrssfeed.com) seems intresting. 
- [ ] Integrate with Itunes API for various crap
- [ ] YoutubeFeeds
