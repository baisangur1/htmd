use crate::{
    Context, ContextKind, Element,
    element_handler::{
        HandlerResult, Handlers,
        element_util::{
            serialize_element, serialize_if_extra_attrs,
            serialize_inline_element_with_walked_children, serialize_when_faithful,
        },
    },
    options::TranslationMode,
    util::{
        text::{StripWhitespace, concat_strings},
        unicode::is_unicode_punctuation,
    },
};

pub(super) fn emphasis_handler(
    handlers: &dyn Handlers,
    element: &Element,
    marker: &str,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    // Emphasis is an inline: its children begin an inline context, though text
    // inside a `<pre>` stays literal.
    let children_context = Context {
        kind: ContextKind::Inline,
        ..element.context
    };
    let walked_children = handlers.walk_children_content(element.node, children_context);
    // An emphasis is never a block element, so `serialize_element` writes it
    // inline from children walked in `Context::INLINE`.
    let serialize = || {
        if children_context == Context::INLINE {
            serialize_inline_element_with_walked_children(element, &walked_children)
        } else {
            serialize_element(handlers, element)
        }
    };
    // Note: this is whitespace, NOT document whitespace, per the
    // [Commonmark spec](https://spec.commonmark.org/0.31.2/#emphasis-and-strong-emphasis).
    let (content, leading_whitespace) = walked_children.strip_leading_whitespace();
    let (content, trailing_whitespace) = content.strip_trailing_whitespace();
    // Content which is whitespace, or nothing at all, leaves nothing to
    // emphasize, and CommonMark cannot spell an emphasis empty. See the
    // "Emphasis and strong emphasis" section of `translation_approach.md`.
    serialize_when_faithful!(handlers, content.is_empty(), serialize());
    // The element goes, but the whitespace it held still separates the text
    // around it. The trailing run is empty, the leading one having taken all of
    // the content.
    if content.is_empty() {
        return Some(leading_whitespace.unwrap_or("").into());
    }
    // See the "Emphasis and strong emphasis" section of
    // `translation_approach.md`.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && (content.starts_with(is_unicode_punctuation)
            || content.ends_with(is_unicode_punctuation))
    {
        return Some(HandlerResult::html(serialize()));
    }

    let content = concat_strings!(
        leading_whitespace.unwrap_or(""),
        marker,
        content,
        marker,
        trailing_whitespace.unwrap_or("")
    );
    Some(content.into())
}
