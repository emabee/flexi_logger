use super::state::{timestamp_from_ts_infix, InfixFormat};

#[derive(Clone)]
pub(crate) enum InfixFilter {
    Timstmps(InfixFormat),
    Numbrs,
    #[cfg(test)]
    StartsWth(String),
    Equls(String),
    None,
}
impl InfixFilter {
    pub(crate) fn filter_infix(&self, infix: &str) -> bool {
        match self {
            InfixFilter::Timstmps(infix_format) => {
                timestamp_from_ts_infix(infix, infix_format).is_ok()
            }
            InfixFilter::Numbrs => {
                if infix.len() > 2 {
                    let mut chars = infix.chars();
                    chars.next().unwrap() == 'r' && chars.next().unwrap().is_ascii_digit()
                } else {
                    false
                }
            }
            #[cfg(test)]
            InfixFilter::StartsWth(s) => infix.starts_with(s),
            InfixFilter::Equls(s) => infix.eq(s),
            InfixFilter::None => false,
        }
    }
}
