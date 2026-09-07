use ai_token_meter_windows::brand_links::BrandLink;

#[test]
fn accepts_only_the_two_fixed_brand_link_identifiers() {
    assert_eq!(
        serde_json::from_str::<BrandLink>(r#""twitter""#).unwrap(),
        BrandLink::Twitter
    );
    assert_eq!(
        serde_json::from_str::<BrandLink>(r#""github""#).unwrap(),
        BrandLink::Github
    );
    assert!(serde_json::from_str::<BrandLink>(r#""file:///tmp/other""#).is_err());
    assert!(serde_json::from_str::<BrandLink>(r#""https://example.com""#).is_err());
}

#[test]
fn resolves_identifiers_to_fixed_https_destinations() {
    assert_eq!(BrandLink::Twitter.url(), "https://twitter.com/MillerPanYue");
    assert_eq!(
        BrandLink::Github.url(),
        "https://github.com/sljzdotcom/AI-Token-Meter"
    );
}
