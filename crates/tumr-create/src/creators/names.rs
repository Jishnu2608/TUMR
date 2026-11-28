//! Project names in the forms each framework's tool accepts. The name passed validation
//! (`request::check_name`: starts with a letter; letters, digits, spaces, `_`, `-`).

fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
}

/// `HelloApp` for "Hello app" (React Native's project name: letters and digits only).
#[must_use]
pub fn identifier(name: &str) -> String {
    words(name)
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_ascii_uppercase().to_string() + c.as_str()
            })
        })
        .collect()
}

/// `hello_app` (a Dart package name: lower case, digits, `_`).
#[must_use]
pub fn snake(name: &str) -> String {
    words(name)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// `hello-app` (an npm package name and an Expo slug).
#[must_use]
pub fn slug(name: &str) -> String {
    words(name)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms() {
        assert_eq!(identifier("Hello app"), "HelloApp");
        assert_eq!(identifier("my-app 2"), "MyApp2");
        assert_eq!(identifier("calc_Pro"), "CalcPro");
        assert_eq!(snake("Hello App"), "hello_app");
        assert_eq!(snake("My-App_2"), "my_app_2");
        assert_eq!(slug("Hello App"), "hello-app");
        assert_eq!(slug("A  B"), "a-b");
    }
}
