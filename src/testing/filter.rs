/// `jvmfast test --filter <spec>` (seção 8.1): `"tag:fast"` filtra por tag
/// JUnit, qualquer outra coisa é um glob de nome de classe (`"*.UserTest"`).
/// Isso é vocabulário próprio do jvm-fast sobre o Console Launcher real —
/// traduzido para `--include-tag`/`--include-classname` na hora de montar o
/// comando (`console::run`), nunca repassado cru.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFilter {
    ClassNameGlob(String),
    Tag(String),
}

pub fn parse_filter(spec: &str) -> TestFilter {
    match spec.strip_prefix("tag:") {
        Some(tag) => TestFilter::Tag(tag.to_string()),
        None => TestFilter::ClassNameGlob(spec.to_string()),
    }
}

/// Converte um glob simples (só `*` como wildcard) para a regex ancorada
/// que `--include-classname` do Console Launcher espera — escapa todo
/// caractere especial de regex exceto `*`, que vira `.*`. `pub` (não só
/// interno de `console::run`) para ser testável diretamente, seguindo a
/// convenção deste projeto de nunca usar `#[cfg(test)]` inline.
pub fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::with_capacity(glob.len() + 2);
    regex.push('^');
    for ch in glob.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            other => regex.push(other),
        }
    }
    regex.push('$');
    regex
}
