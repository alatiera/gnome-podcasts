use rss::Channel;


pub fn foo() {

    let channel = Channel::from_url("https://feeds.feedburner.com/InterceptedWithJeremyScahill").unwrap();
    println!("{:#?}", channel);
}