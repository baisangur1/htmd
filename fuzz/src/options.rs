//! Decoding of a fuzz input; see "Input decoding".

use htmd::options::{
    BrStyle, BulletListMarker, CodeBlockFence, CodeBlockStyle, HeadingStyle, HrStyle,
    LinkReferenceStyle, LinkStyle, Options, TranslationMode,
};

const PREFIX_LEN: usize = 13;

/// Which fields of `Options` a target lets the input choose.
pub enum Target {
    Convert,
    RoundTrip,
}

/// Options and HTML decoded from a fuzz input.
pub struct FuzzInput<'a> {
    pub options: Options,
    pub scripting_enabled: bool,
    pub html_bytes: &'a [u8],
}

/// Decode `data`, or `None` if it is shorter than the option prefix.
pub fn decode(data: &[u8], target: Target) -> Option<FuzzInput<'_>> {
    let (prefix, html_bytes) = data.split_at_checked(PREFIX_LEN)?;
    let default = Options::default();
    let flip = |value: bool, byte: u8| value ^ (byte % 2 == 1);
    let mut options = Options {
        heading_style: pick(
            &[HeadingStyle::Atx, HeadingStyle::Setex],
            default.heading_style,
            prefix[0],
        ),
        hr_style: pick(
            &[HrStyle::Dashes, HrStyle::Asterisks, HrStyle::Underscores],
            default.hr_style,
            prefix[1],
        ),
        br_style: pick(
            &[BrStyle::TwoSpaces, BrStyle::Backslash],
            default.br_style,
            prefix[2],
        ),
        link_style: pick(
            &[
                LinkStyle::Inlined,
                LinkStyle::InlinedPreferAutolinks,
                LinkStyle::Referenced,
            ],
            default.link_style,
            prefix[3],
        ),
        link_reference_style: pick(
            &[
                LinkReferenceStyle::Full,
                LinkReferenceStyle::Collapsed,
                LinkReferenceStyle::Shortcut,
            ],
            default.link_reference_style,
            prefix[4],
        ),
        code_block_style: pick(
            &[CodeBlockStyle::Indented, CodeBlockStyle::Fenced],
            default.code_block_style,
            prefix[5],
        ),
        code_block_fence: pick(
            &[CodeBlockFence::Tildes, CodeBlockFence::Backticks],
            default.code_block_fence,
            prefix[6],
        ),
        bullet_list_marker: pick(
            &[BulletListMarker::Asterisk, BulletListMarker::Dash],
            default.bullet_list_marker,
            prefix[7],
        ),
        ul_bullet_spacing: default.ul_bullet_spacing.wrapping_add(prefix[8]),
        ol_number_spacing: default.ol_number_spacing.wrapping_add(prefix[9]),
        preformatted_code: flip(default.preformatted_code, prefix[10]),
        translation_mode: pick(
            &[TranslationMode::Pure, TranslationMode::Faithful],
            default.translation_mode,
            prefix[11],
        ),
    };
    if let Target::RoundTrip = target {
        options.translation_mode = TranslationMode::Faithful;
        options.preformatted_code = true;
        options.link_reference_style = LinkReferenceStyle::Full;
    }
    Some(FuzzInput {
        options,
        scripting_enabled: flip(true, prefix[12]),
        html_bytes,
    })
}

/// The variant `(d + b) mod n` of `variants`, listed in declaration order.
fn pick<T: Copy + PartialEq>(variants: &[T], default: T, byte: u8) -> T {
    let default_index = variants.iter().position(|&v| v == default).unwrap();
    variants[(default_index + byte as usize) % variants.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(prefix: [u8; PREFIX_LEN], target: Target) -> FuzzInput<'static> {
        decode(prefix.to_vec().leak(), target).unwrap()
    }

    #[test]
    fn zero_prefix_is_default() {
        let decoded = input([0; PREFIX_LEN], Target::Convert);
        assert_eq!(decoded.options, Options::default());
        assert!(decoded.scripting_enabled);
    }

    #[test]
    fn html_follows_prefix() {
        assert!(decode(&[0; PREFIX_LEN - 1], Target::Convert).is_none());
        let data = [[0; PREFIX_LEN].as_slice(), b"<p>a</p>"].concat();
        assert_eq!(
            decode(&data, Target::Convert).unwrap().html_bytes,
            b"<p>a</p>"
        );
    }

    #[test]
    fn enums_wrap_from_default() {
        let mut prefix = [0; PREFIX_LEN];
        prefix[1] = 1;
        prefix[5] = 3;
        let options = input(prefix, Target::Convert).options;
        assert_eq!(options.hr_style, HrStyle::Underscores);
        assert_eq!(options.code_block_style, CodeBlockStyle::Indented);
        prefix[1] = 2;
        assert_eq!(
            input(prefix, Target::Convert).options.hr_style,
            HrStyle::Dashes
        );
    }

    #[test]
    fn spacing_wraps() {
        let mut prefix = [0; PREFIX_LEN];
        prefix[8] = 254;
        prefix[9] = 255;
        let options = input(prefix, Target::Convert).options;
        assert_eq!(options.ul_bullet_spacing, 1);
        assert_eq!(options.ol_number_spacing, 1);
    }

    #[test]
    fn odd_bytes_flip_booleans() {
        let mut prefix = [0; PREFIX_LEN];
        prefix[10] = 3;
        prefix[12] = 255;
        let decoded = input(prefix, Target::Convert);
        assert!(decoded.options.preformatted_code);
        assert!(!decoded.scripting_enabled);
        prefix[10] = 2;
        assert!(!input(prefix, Target::Convert).options.preformatted_code);
    }

    #[test]
    fn round_trip_fixes_three_fields() {
        let mut prefix = [1; PREFIX_LEN];
        prefix[11] = 0;
        let options = input(prefix, Target::RoundTrip).options;
        assert_eq!(options.translation_mode, TranslationMode::Faithful);
        assert!(options.preformatted_code);
        assert_eq!(options.link_reference_style, LinkReferenceStyle::Full);
        assert_eq!(options.heading_style, HeadingStyle::Setex);
    }
}
