#[derive(Debug)]
pub enum Token {
    Tag(String),
    Text(String),
}

pub fn lex(body: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut in_tag = false;
    let mut in_entity = false;
    let mut buffer = String::new();
    let mut entity = String::new();
    for c in body.chars() {
        if c == '<' {
            in_tag = true;
            if in_entity {
                buffer.push('&');
                buffer.push_str(&entity);
                entity.clear();
                in_entity = false;
            }
            if !buffer.is_empty() {
                out.push(Token::Text(buffer.clone()));
                buffer.clear();
            }
        } else if c == '>' {
            in_tag = false;
            out.push(Token::Tag(buffer.to_lowercase()));
            buffer.clear();
        } else if c == '&' && !in_tag {
            in_entity = true;
        } else if in_entity && !in_tag {
            if c.is_ascii_alphanumeric() {
                entity.push(c);
            } else if c == ';' {
                if let Some(translated) = translate_entity(&entity) {
                    buffer.push_str(translated);
                } else {
                    buffer.push('&');
                    buffer.push_str(&entity);
                    buffer.push(';');
                }
                in_entity = false;
                entity.clear();
            } else {
                buffer.push('&');
                buffer.push_str(&entity);
                in_entity = false;
                entity.clear();
            }
        } else {
            buffer.push(c);
        }
    }
    if !in_tag {
        if in_entity {
            buffer.push('&');
            buffer.push_str(&entity);
        }
        if !buffer.is_empty() {
            out.push(Token::Text(buffer));
        }
    }
    out
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
