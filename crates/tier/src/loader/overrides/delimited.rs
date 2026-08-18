#[derive(Debug, Clone, PartialEq, Eq)]
struct DelimitedField {
    value: String,
    quoted: bool,
}

pub(super) fn parse_csv_fields(raw: &str) -> Result<Vec<String>, String> {
    parse_delimited_fields(raw, ',').map(filter_empty_unquoted_fields)
}

pub(super) fn parse_key_value_fields(raw: &str) -> Result<Vec<String>, String> {
    parse_key_value_delimited_fields(raw, ',').map(filter_empty_unquoted_fields)
}

fn filter_empty_unquoted_fields(fields: Vec<DelimitedField>) -> Vec<String> {
    fields
        .into_iter()
        .filter(|field| field.quoted || !field.value.is_empty())
        .map(|field| field.value)
        .collect()
}

fn parse_delimited_fields(raw: &str, delimiter: char) -> Result<Vec<DelimitedField>, String> {
    let mut fields = Vec::new();
    let mut chars = raw.chars().peekable();

    loop {
        consume_horizontal_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }

        let field = if chars.peek() == Some(&'"') {
            chars.next();
            let value = parse_quoted_field(&mut chars, "csv")?;
            consume_horizontal_whitespace(&mut chars);
            DelimitedField {
                value,
                quoted: true,
            }
        } else {
            DelimitedField {
                value: parse_unquoted_field(&mut chars, delimiter)
                    .trim()
                    .to_owned(),
                quoted: false,
            }
        };
        fields.push(field);

        match chars.peek() {
            Some(ch) if *ch == delimiter => {
                chars.next();
            }
            Some(ch) => {
                return Err(format!(
                    "invalid csv field separator `{ch}`, expected `{delimiter}`"
                ));
            }
            None => break,
        }
    }

    Ok(fields)
}

fn parse_key_value_delimited_fields(
    raw: &str,
    delimiter: char,
) -> Result<Vec<DelimitedField>, String> {
    let mut fields = Vec::new();
    let mut chars = raw.chars().peekable();
    let mut value = String::new();
    let mut quoted = false;

    while let Some(ch) = chars.next() {
        if ch == delimiter {
            fields.push(DelimitedField {
                value: value.trim().to_owned(),
                quoted,
            });
            value.clear();
            quoted = false;
            continue;
        }

        if ch == '"' {
            quoted = true;
            value.push_str(&parse_quoted_field(&mut chars, "key_value_map")?);
            continue;
        }

        value.push(ch);
    }

    fields.push(DelimitedField {
        value: value.trim().to_owned(),
        quoted,
    });

    Ok(fields)
}

fn parse_quoted_field<I>(
    chars: &mut std::iter::Peekable<I>,
    context: &str,
) -> Result<String, String>
where
    I: Iterator<Item = char>,
{
    let mut value = String::new();
    loop {
        match chars.next() {
            Some('"') if chars.peek() == Some(&'"') => {
                chars.next();
                value.push('"');
            }
            Some('"') => return Ok(value),
            Some(ch) => value.push(ch),
            None => return Err(format!("unterminated quoted {context} field")),
        }
    }
}

fn parse_unquoted_field<I>(chars: &mut std::iter::Peekable<I>, delimiter: char) -> String
where
    I: Iterator<Item = char>,
{
    let mut value = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch == delimiter {
            break;
        }
        chars.next();
        value.push(ch);
    }
    value
}

fn consume_horizontal_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while matches!(chars.peek(), Some(' ' | '\t')) {
        chars.next();
    }
}
