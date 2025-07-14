pub trait TagName {
    const TAG_NAME: &'static str;
    const NAMESPACE: Option<&'static str>;
}
