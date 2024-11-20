pub fn show(body: &str) {
    let mut in_tag = false;
    for c in body.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            print!("{}", c);
        }
    }
}
