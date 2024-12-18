fn main() {
    let mut s = "我爱你!中国".to_string();
    let r = s.as_mut();

    if let Some((s1, s2)) = split(r, '!') {
        println!("s1: {s1}, s2: {s2}");
    }
}

fn split(s: &str, sep: char) -> Option<(&str, &str)> {
    let pos = s.find(sep);
    pos.map(|pos| {
        let len = s.len();
        let sep_len = sep.len_utf8();

        // SAFETY: pos是find得到的, 它位于字符的边界处, 同样pos + sep_len也是如此
        // 所以一下的代码是安全的
        unsafe { (s.get_unchecked(0..pos), s.get_unchecked(pos + sep_len..len)) }
    })
}
