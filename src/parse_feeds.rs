use rss::Channel;
use models;

pub fn parse_podcast<'a>(podcast_chan: Channel) -> models::NewPodcast<'a> {

    let foo = models::NewPodcast {
        title: "foo",
        uri: "foo",
        link: Some("foo"),
        description: Some("foo"),

        // need to get it from reqwest probably
        last_modified: Some("foo"),
        http_etag: Some("foo"),

        image_uri: Some("foo"),
        image_local: Some("foo"),
    };
    foo
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use rss::Channel;

    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_parse_podcast() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        // println!("{:#?}", channel);
        let descr = "The people behind The Intercept’s fearless reporting and incisive commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss the crucial issues of our time: national security, civil liberties, foreign policy, and criminal justice.  Plus interviews with artists, thinkers, and newsmakers who challenge our preconceptions about the world we live in.";
        let pd = parse_podcast(channel);

        assert_eq!(pd.title, "Intercepted with Jeremy Scahill");
        assert_eq!(pd.link, Some("https://theintercept.com/podcasts"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(pd.last_modified, None);
        assert_eq!(pd.http_etag, None);
        assert_eq!(pd.image_uri, None);
        assert_eq!(pd.image_local, None);

    }

}