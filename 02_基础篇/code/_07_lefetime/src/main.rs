// pub fn strtok<'a>(s: &mut &'a str, delimiter: char) -> &'a str {
//     if let Some(i) = s.find(delimiter) {
//         let prefix = &s[..i];
//         let suffix = &s[(i + delimiter.len_utf8())..];
//         *s = suffix;
//         prefix
//     } else {
//         let prefix = *s;
//         *s = "";
//         prefix
//     }
// }

// fn main() {
//     let s = "hello world".to_string();
//     let mut s1 = s.as_str();
//     let hello = strtok(&mut s1, ' ');
//     println!("hello is: {}, s1: {}, s: {}", hello, s1, s);
// }

// fn main() {
//     let mut s = "hello world";
//     let s1 = &mut s;
//     *s1 = "hello world";
//     println!("s: {s}");
// }

pub fn strtok<'a>(s: &'a mut &str, delimiter: char) -> &'a str {
    if let Some(i) = s.find(delimiter) {
        let prefix = &s[..i];
        let suffix = &s[(i + delimiter.len_utf8())..];
        *s = suffix;
        prefix
    } else {
        let prefix = *s;
        *s = "";
        prefix
    }
}

fn main() {
    let s = "hello world".to_string();
    let mut s1 = s.as_str();
    let hello = strtok(&mut s1, ' ');
    println!("hello is: {}, s1: {}, s: {}", hello, s1, s);
}
