use syn::{
    Fields, FieldsNamed, File, Item, ItemEnum, ItemStruct, Meta, MetaList, NestedMeta, Variant,
};

pub trait Extract {
    fn extract_message_with_fields_named(&mut self, _: &ItemStruct, _: &FieldsNamed) {}

    fn extract_nested_message_with_fields_named(
        &mut self,
        _: &ItemEnum,
        _: &Variant,
        _: &FieldsNamed,
    ) {
    }

    fn extract_nested_message_with_fields_unit(&mut self, _: &ItemEnum, _: &Variant) {}

    fn extract_one_of(&mut self, _: &ItemEnum) {}

    fn extract_enumerator(&mut self, _: &ItemEnum) {}
}

pub fn extract_nested_message<T: Extract + ?Sized>(
    e: &mut T,
    item_enum: &ItemEnum,
    variant: &Variant,
) {
    match &variant.fields {
        Fields::Named(fields_named) => {
            e.extract_nested_message_with_fields_named(item_enum, variant, fields_named);
        }
        Fields::Unit => {
            e.extract_nested_message_with_fields_unit(item_enum, variant);
        }
        _ => {
            panic!(
                "only unit and 'struct' with named fields can be converted to nested 'message': \"{:?}\"",
                item_enum.ident
            );
        }
    }
}

pub fn extract_message<T: Extract + ?Sized>(e: &mut T, item_struct: &ItemStruct) {
    fn filter_field(field: &syn::Field) -> bool {
        if let syn::Visibility::Public(_) = field.vis {
            true
        } else {
            syn_util::contains_attribute(&field.attrs, &["protobuf_gen", "expose"])
        }
    }

    if let syn::Fields::Named(fields_named) = &item_struct.fields {
        let fields_named = FieldsNamed {
            named: fields_named
                .named
                .iter()
                .cloned()
                .filter(filter_field)
                .collect(),
            ..fields_named.clone()
        };
        if fields_named.named.len() > 0 {
            e.extract_message_with_fields_named(item_struct, &fields_named);
            return;
        }
    }

    panic!(
        "only 'struct' with named fields can be converted to 'message': \"{:?}\"",
        item_struct.ident
    );
}

fn collect_items(file: &File) -> Vec<&Item> {
    let is_protobuf_gen = |nested_meta: &NestedMeta| {
        if let NestedMeta::Meta(Meta::Word(meta_word)) = nested_meta {
            meta_word == "ProtobufGen"
        } else {
            false
        }
    };

    file.items
        .iter()
        .filter(|item| match item {
            Item::Struct(ItemStruct { attrs, .. }) | Item::Enum(ItemEnum { attrs, .. }) => {
                attrs.iter().any(|attr| {
                    if let Meta::List(MetaList { ident, nested, .. }) = attr.parse_meta().unwrap() {
                        ident == "derive"
                            && nested
                                .iter()
                                .any(|nested_meta| is_protobuf_gen(nested_meta))
                    } else {
                        false
                    }
                })
            }
            _ => false,
        })
        .collect()
}

pub fn extract_from_file<T: Extract + ?Sized>(e: &mut T, file: &File) {
    let items = collect_items(file);

    for item in items {
        match item {
            Item::Struct(item_struct) => {
                extract_message(e, item_struct);
            }
            Item::Enum(item_enum) => {
                if item_enum.variants.iter().all(|v| {
                    if let Fields::Unit = v.fields {
                        true
                    } else {
                        false
                    }
                }) {
                    e.extract_enumerator(item_enum);
                } else {
                    e.extract_one_of(item_enum);

                    for variant in &item_enum.variants {
                        extract_nested_message(e, item_enum, variant);
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}
