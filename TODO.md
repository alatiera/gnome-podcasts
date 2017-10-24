## TODOs:

**General:**

- [ ] Add CONTRIBUTING.md
- [ ] Write docs


## Priorities:

**Would be nice:**

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

- [ ] Not sure how bad-utf8 is handled in rss crate
- [ ] Icons && install stuff && flatpak


**DB changes:**

- [ ] Db episodes: add watched field
- [ ] Mark episodes/podcast for archival
- [ ] Podcast deletion
- [ ] Download cleaner
- [ ] Mark stuff as Favorite. Maybe auto-archive favorites?
- [ ] New episode notifier on podcast_flowbox_child, like the one vocal has


## Secondary:

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
