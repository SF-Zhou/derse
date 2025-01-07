#[derive(derse::Serialize)]
struct T1;

#[derive(derse::Serialize)]
struct T2(i32);

#[derive(derse::Serialize)]
struct T3(i32, String);

#[derive(derse::Serialize)]
struct T4{
    idx: i32,
    name: String
}

fn main() {}
