pub fn lex(body: &str) -> String {
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity = String::new();
    let mut text = String::new();
    for c in body.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if c == '&' {
            in_entity = true;
        } else if in_entity && !in_tag {
            if c.is_ascii_alphanumeric() {
                entity.push(c);
            } else if c == ';' {
                if let Some(translated) = translate_entity(&entity) {
                    text.push_str(translated);
                } else {
                    text.push('&');
                    text.push_str(&entity);
                    text.push(';');
                }
                in_entity = false;
                entity.clear();
            } else {
                text.push('&');
                text.push_str(&entity);
                in_entity = false;
                entity.clear();
            }
        } else if !in_tag {
            text.push(c);
        }
    }
    text
}

fn translate_entity(entity: &str) -> Option<&str> {
    Some(match entity {
        "gt" => ">",
        "lt" => "<",
        "amp" => "&",
        "hellip" => "…",
        "rsquo" => "’",
        "ldquo" => "“",
        "rdquo" => "”",
        _ => return None,
    })
}
