#[cfg(test)]
mod tests {
    // use futures::future::result;
    use tokio_core::reactor::Core;
    use hyper::Client;
    use hyper_tls::HttpsConnector;

    use database::truncate_db;
    use Source;

    #[test]
    fn test_bar() {
        truncate_db().unwrap();

        let mut core = Core::new().unwrap();
        let mut client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        let url = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";
        let mut source = Source::from_url(url).unwrap();

        let feed = source.into_fututre_feed(&mut client, false);

        let f = core.run(feed).unwrap();

        println!("{:?}", f);
    }
}
