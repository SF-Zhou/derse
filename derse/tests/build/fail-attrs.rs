#[derive(derse::Deserialize)]
struct Conflicting {
    #[derse(required, default = "String::new")]
    value: String,
}

#[derive(derse::Deserialize)]
struct Duplicate {
    #[derse(required)]
    #[derse(required)]
    value: String,
}

#[derive(derse::Deserialize)]
struct Unknown {
    #[derse(optional)]
    value: String,
}

#[derive(derse::Deserialize)]
struct InvalidDefault {
    #[derse(default = "42")]
    value: String,
}

#[derive(derse::Deserialize)]
#[derse(required)]
struct OnContainer;

#[derive(derse::Deserialize)]
enum OnVariant {
    #[derse(required)]
    Value,
}

#[derive(derse::Serialize)]
struct SerializeAlsoValidates {
    #[derse(unknown)]
    value: String,
}

fn main() {}
