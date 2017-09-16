use rss::Channel;
use models;

pub fn parse_podcast<'a>(pd_chan: &'a Channel, uri: &'a str) -> models::NewPodcast<'a> {
    let title = pd_chan.title();

    // need to get it from reqwest probably
    // I dont think uri can be consinstantly infered from the Channel
    // TODO: Add etag support
    let last_modified = None;
    let http_etag = None;

    let link = Some(pd_chan.link());
    let description = Some(pd_chan.description());

    let image_uri = match pd_chan.image() {
        Some(foo) => Some(foo.url()),
        None => None,
    };

    models::NewPodcast {
        title,
        uri,
        link,
        description,
        last_modified,
        http_etag,
        image_uri,
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use rss::Channel;

    use super::*;

    #[test]
    fn test_parse_podcast() {
        // Intercepted feed
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";

        // println!("{:#?}", channel);
        let descr = "The people behind The Intercept’s fearless reporting and incisive commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss the crucial issues of our time: national security, civil liberties, foreign policy, and criminal justice.  Plus interviews with artists, thinkers, and newsmakers who challenge our preconceptions about the world we live in.";
        let pd = parse_podcast(&channel, uri);

        assert_eq!(pd.title, "Intercepted with Jeremy Scahill");
        // assert_eq!(
        //     pd.uri,
        //     "https://feeds.feedburner.com/InterceptedWithJeremyScahill"
        // );
        assert_eq!(pd.link, Some("https://theintercept.com/podcasts"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(pd.last_modified, None);
        assert_eq!(pd.http_etag, None);
        assert_eq!(pd.image_uri, None);


        // Linux Unplugged Feed
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "http://feeds.feedburner.com/linuxunplugged";

        // println!("{:#?}", channel);
        let descr = "An open show powered by community LINUX Unplugged takes the best attributes of open collaboration and focuses them into a weekly lifestyle show about Linux.";
        let pd = parse_podcast(&channel, uri);

        assert_eq!(pd.title, "LINUX Unplugged Podcast");
        assert_eq!(pd.link, Some("http://www.jupiterbroadcasting.com/"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(pd.last_modified, None);
        assert_eq!(pd.http_etag, None);
        assert_eq!(
            pd.image_uri,
            Some("http://michaeltunnell.com/images/linux-unplugged.jpg")
        );


        // The Breakthrough Feed
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "http://feeds.propublica.org/propublica/main";

        // println!("{:#?}", channel);
        let descr = "Latest Articles and Investigations from ProPublica, an independent, non-profit newsroom that produces investigative journalism in the public interest.";
        let pd = parse_podcast(&channel, uri);

        assert_eq!(pd.title, "Articles and Investigations - ProPublica");
        assert_eq!(pd.link, Some("https://www.propublica.org/feeds/54Ghome"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(pd.last_modified, None);
        assert_eq!(pd.http_etag, None);
        assert_eq!(pd.image_uri, Some("https://assets.propublica.org/propublica-rss-logo.png"));
    }

}