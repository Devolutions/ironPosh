pub trait TagName {
    fn tag_name(&self) -> &'static str;
    fn namespace(&self) -> Option<&'static str>;
}
