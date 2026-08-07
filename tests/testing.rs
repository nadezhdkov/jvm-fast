use jvmfast::testing::{glob_to_regex, parse_filter, TestFilter};

#[test]
fn parse_filter_recognizes_tag_prefix() {
    assert_eq!(
        parse_filter("tag:fast"),
        TestFilter::Tag("fast".to_string())
    );
}

#[test]
fn parse_filter_treats_anything_else_as_a_classname_glob() {
    assert_eq!(
        parse_filter("*.UserTest"),
        TestFilter::ClassNameGlob("*.UserTest".to_string())
    );
}

#[test]
fn parse_filter_does_not_treat_tag_in_the_middle_as_the_prefix() {
    assert_eq!(
        parse_filter("com.acme.tag:fast.Test"),
        TestFilter::ClassNameGlob("com.acme.tag:fast.Test".to_string())
    );
}

#[test]
fn glob_to_regex_translates_star_to_dot_star_and_anchors() {
    assert_eq!(glob_to_regex("*.UserTest"), "^.*\\.UserTest$");
}

#[test]
fn glob_to_regex_escapes_regex_special_characters() {
    assert_eq!(glob_to_regex("Foo(Bar)"), "^Foo\\(Bar\\)$");
}

#[test]
fn glob_to_regex_matches_exact_class_name_with_no_wildcard() {
    assert_eq!(
        glob_to_regex("com.acme.UserTest"),
        "^com\\.acme\\.UserTest$"
    );
}
