use proc_macro2::Span;

pub(crate) struct AttrSpansValue<T> {
    pub(crate) attr_spans: Vec<Span>,
    pub(crate) value: T,
}
