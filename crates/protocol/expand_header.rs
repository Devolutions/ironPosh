pub mod header {
    use crate::{cores::*, ws_addressing::AddressValue, ws_management::OptionSetValue};
    pub struct SoapHeaders<'a> {
        /// WS-Addressing headers
        #[builder(default, setter(into, strip_option))]
        pub to: Option<Tag<'a, Text<'a>, To>>,
        #[builder(default, setter(into, strip_option))]
        pub action: Option<Tag<'a, Text<'a>, Action>>,
        #[builder(default, setter(into, strip_option))]
        pub reply_to: Option<Tag<'a, AddressValue<'a>, ReplyTo>>,
        #[builder(default, setter(into, strip_option))]
        pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
        #[builder(default, setter(into, strip_option))]
        pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,
        /// WS-Management headers
        #[builder(default, setter(into, strip_option))]
        pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
        #[builder(default, setter(into, strip_option))]
        pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
        #[builder(default, setter(into, strip_option))]
        pub locale: Option<Tag<'a, Text<'a>, Locale>>,
        #[builder(default, setter(into, strip_option))]
        pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
        #[builder(default, setter(into, strip_option))]
        pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
        #[builder(default, setter(into, strip_option))]
        pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
        #[builder(default, setter(into, strip_option))]
        pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
        #[builder(default, setter(into, strip_option))]
        pub option_set: Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
        #[builder(default, setter(into, strip_option))]
        pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
        #[builder(default, setter(into, strip_option))]
        pub compression_type: Option<Tag<'a, Text<'a>, CompressionType>>,
    }
    #[automatically_derived]
    impl<'a> ::core::fmt::Debug for SoapHeaders<'a> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "to",
                "action",
                "reply_to",
                "message_id",
                "relates_to",
                "resource_uri",
                "max_envelope_size",
                "locale",
                "data_locale",
                "session_id",
                "operation_id",
                "sequence_id",
                "option_set",
                "operation_timeout",
                "compression_type",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.to,
                &self.action,
                &self.reply_to,
                &self.message_id,
                &self.relates_to,
                &self.resource_uri,
                &self.max_envelope_size,
                &self.locale,
                &self.data_locale,
                &self.session_id,
                &self.operation_id,
                &self.sequence_id,
                &self.option_set,
                &self.operation_timeout,
                &&self.compression_type,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "SoapHeaders",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for SoapHeaders<'a> {
        #[inline]
        fn clone(&self) -> SoapHeaders<'a> {
            SoapHeaders {
                to: ::core::clone::Clone::clone(&self.to),
                action: ::core::clone::Clone::clone(&self.action),
                reply_to: ::core::clone::Clone::clone(&self.reply_to),
                message_id: ::core::clone::Clone::clone(&self.message_id),
                relates_to: ::core::clone::Clone::clone(&self.relates_to),
                resource_uri: ::core::clone::Clone::clone(&self.resource_uri),
                max_envelope_size: ::core::clone::Clone::clone(&self.max_envelope_size),
                locale: ::core::clone::Clone::clone(&self.locale),
                data_locale: ::core::clone::Clone::clone(&self.data_locale),
                session_id: ::core::clone::Clone::clone(&self.session_id),
                operation_id: ::core::clone::Clone::clone(&self.operation_id),
                sequence_id: ::core::clone::Clone::clone(&self.sequence_id),
                option_set: ::core::clone::Clone::clone(&self.option_set),
                operation_timeout: ::core::clone::Clone::clone(&self.operation_timeout),
                compression_type: ::core::clone::Clone::clone(&self.compression_type),
            }
        }
    }
    #[automatically_derived]
    impl<'a> SoapHeaders<'a> {
        /**
                Create a builder for building `SoapHeaders`.
                On the builder, call `.to(...)`(optional), `.action(...)`(optional), `.reply_to(...)`(optional), `.message_id(...)`(optional), `.relates_to(...)`(optional), `.resource_uri(...)`(optional), `.max_envelope_size(...)`(optional), `.locale(...)`(optional), `.data_locale(...)`(optional), `.session_id(...)`(optional), `.operation_id(...)`(optional), `.sequence_id(...)`(optional), `.option_set(...)`(optional), `.operation_timeout(...)`(optional), `.compression_type(...)`(optional) to set the values of the fields.
                Finally, call `.build()` to create the instance of `SoapHeaders`.
                */
        #[allow(dead_code, clippy::default_trait_access)]
        pub fn builder() -> SoapHeadersBuilder<
            'a,
            ((), (), (), (), (), (), (), (), (), (), (), (), (), (), ()),
        > {
            SoapHeadersBuilder {
                fields: ((), (), (), (), (), (), (), (), (), (), (), (), (), (), ()),
                phantom: ::core::default::Default::default(),
            }
        }
    }
    #[must_use]
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    pub struct SoapHeadersBuilder<
        'a,
        TypedBuilderFields = ((), (), (), (), (), (), (), (), (), (), (), (), (), (), ()),
    > {
        fields: TypedBuilderFields,
        phantom: ::core::marker::PhantomData<(::core::marker::PhantomData<&'a ()>)>,
    }
    #[automatically_derived]
    impl<'a, TypedBuilderFields> Clone for SoapHeadersBuilder<'a, TypedBuilderFields>
    where
        TypedBuilderFields: Clone,
    {
        #[allow(clippy::default_trait_access)]
        fn clone(&self) -> Self {
            Self {
                fields: self.fields.clone(),
                phantom: ::core::default::Default::default(),
            }
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            (),
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        /// WS-Addressing headers
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn to(
            self,
            to: impl ::core::convert::Into<Tag<'a, Text<'a>, To>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                (Option<Tag<'a, Text<'a>, To>>,),
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let to = (Some(to.into()),);
            let (
                (),
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_to {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            (Option<Tag<'a, Text<'a>, To>>,),
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field to")]
        /// WS-Addressing headers
        pub fn to(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_to,
        ) -> SoapHeadersBuilder<
            'a,
            (
                (Option<Tag<'a, Text<'a>, To>>,),
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            (),
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn action(
            self,
            action: impl ::core::convert::Into<Tag<'a, Text<'a>, Action>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                (Option<Tag<'a, Text<'a>, Action>>,),
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let action = (Some(action.into()),);
            let (
                to,
                (),
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_action {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            (Option<Tag<'a, Text<'a>, Action>>,),
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field action")]
        pub fn action(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_action,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                (Option<Tag<'a, Text<'a>, Action>>,),
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            (),
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn reply_to(
            self,
            reply_to: impl ::core::convert::Into<Tag<'a, AddressValue<'a>, ReplyTo>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                (Option<Tag<'a, AddressValue<'a>, ReplyTo>>,),
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let reply_to = (Some(reply_to.into()),);
            let (
                to,
                action,
                (),
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_reply_to {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            (Option<Tag<'a, AddressValue<'a>, ReplyTo>>,),
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field reply_to")]
        pub fn reply_to(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_reply_to,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                (Option<Tag<'a, AddressValue<'a>, ReplyTo>>,),
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            (),
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn message_id(
            self,
            message_id: impl ::core::convert::Into<Tag<'a, Text<'a>, MessageID>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                (Option<Tag<'a, Text<'a>, MessageID>>,),
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let message_id = (Some(message_id.into()),);
            let (
                to,
                action,
                reply_to,
                (),
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_message_id {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            (Option<Tag<'a, Text<'a>, MessageID>>,),
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field message_id")]
        pub fn message_id(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_message_id,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                (Option<Tag<'a, Text<'a>, MessageID>>,),
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            (),
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn relates_to(
            self,
            relates_to: impl ::core::convert::Into<Tag<'a, Text<'a>, RelatesTo>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                (Option<Tag<'a, Text<'a>, RelatesTo>>,),
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let relates_to = (Some(relates_to.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                (),
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_relates_to {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            (Option<Tag<'a, Text<'a>, RelatesTo>>,),
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field relates_to")]
        pub fn relates_to(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_relates_to,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                (Option<Tag<'a, Text<'a>, RelatesTo>>,),
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            (),
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        /// WS-Management headers
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn resource_uri(
            self,
            resource_uri: impl ::core::convert::Into<Tag<'a, Text<'a>, ResourceURI>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                (Option<Tag<'a, Text<'a>, ResourceURI>>,),
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let resource_uri = (Some(resource_uri.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                (),
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_resource_uri {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            (Option<Tag<'a, Text<'a>, ResourceURI>>,),
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field resource_uri")]
        /// WS-Management headers
        pub fn resource_uri(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_resource_uri,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                (Option<Tag<'a, Text<'a>, ResourceURI>>,),
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            (),
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn max_envelope_size(
            self,
            max_envelope_size: impl ::core::convert::Into<
                Tag<'a, Text<'a>, MaxEnvelopeSize>,
            >,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                (Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,),
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let max_envelope_size = (Some(max_envelope_size.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                (),
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_max_envelope_size {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            (Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,),
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field max_envelope_size")]
        pub fn max_envelope_size(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_max_envelope_size,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                (Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,),
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            (),
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn locale(
            self,
            locale: impl ::core::convert::Into<Tag<'a, Text<'a>, Locale>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                (Option<Tag<'a, Text<'a>, Locale>>,),
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let locale = (Some(locale.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                (),
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_locale {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            (Option<Tag<'a, Text<'a>, Locale>>,),
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field locale")]
        pub fn locale(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_locale,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                (Option<Tag<'a, Text<'a>, Locale>>,),
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            (),
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn data_locale(
            self,
            data_locale: impl ::core::convert::Into<Tag<'a, Text<'a>, DataLocale>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                (Option<Tag<'a, Text<'a>, DataLocale>>,),
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let data_locale = (Some(data_locale.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                (),
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_data_locale {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            (Option<Tag<'a, Text<'a>, DataLocale>>,),
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field data_locale")]
        pub fn data_locale(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_data_locale,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                (Option<Tag<'a, Text<'a>, DataLocale>>,),
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            (),
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn session_id(
            self,
            session_id: impl ::core::convert::Into<Tag<'a, Text<'a>, SessionId>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                (Option<Tag<'a, Text<'a>, SessionId>>,),
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let session_id = (Some(session_id.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                (),
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_session_id {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            (Option<Tag<'a, Text<'a>, SessionId>>,),
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field session_id")]
        pub fn session_id(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_session_id,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                (Option<Tag<'a, Text<'a>, SessionId>>,),
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            (),
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn operation_id(
            self,
            operation_id: impl ::core::convert::Into<Tag<'a, Text<'a>, OperationID>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                (Option<Tag<'a, Text<'a>, OperationID>>,),
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let operation_id = (Some(operation_id.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                (),
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_operation_id {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            (Option<Tag<'a, Text<'a>, OperationID>>,),
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field operation_id")]
        pub fn operation_id(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_operation_id,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                (Option<Tag<'a, Text<'a>, OperationID>>,),
                __sequence_id,
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            (),
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn sequence_id(
            self,
            sequence_id: impl ::core::convert::Into<Tag<'a, Text<'a>, SequenceId>>,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                (Option<Tag<'a, Text<'a>, SequenceId>>,),
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            let sequence_id = (Some(sequence_id.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                (),
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_sequence_id {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __option_set,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            (Option<Tag<'a, Text<'a>, SequenceId>>,),
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field sequence_id")]
        pub fn sequence_id(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_sequence_id,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                (Option<Tag<'a, Text<'a>, SequenceId>>,),
                __option_set,
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            (),
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn option_set(
            self,
            option_set: impl ::core::convert::Into<
                Tag<'a, OptionSetValue<'a>, OptionSet>,
            >,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                (Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,),
                __operation_timeout,
                __compression_type,
            ),
        > {
            let option_set = (Some(option_set.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                (),
                operation_timeout,
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_option_set {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __operation_timeout,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            (Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,),
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field option_set")]
        pub fn option_set(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_option_set,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                (Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,),
                __operation_timeout,
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            (),
            __compression_type,
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn operation_timeout(
            self,
            operation_timeout: impl ::core::convert::Into<
                Tag<'a, Text<'a>, OperationTimeout>,
            >,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                (Option<Tag<'a, Text<'a>, OperationTimeout>>,),
                __compression_type,
            ),
        > {
            let operation_timeout = (Some(operation_timeout.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                (),
                compression_type,
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_operation_timeout {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __compression_type,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            (Option<Tag<'a, Text<'a>, OperationTimeout>>,),
            __compression_type,
        ),
    > {
        #[deprecated(note = "Repeated field operation_timeout")]
        pub fn operation_timeout(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_operation_timeout,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                (Option<Tag<'a, Text<'a>, OperationTimeout>>,),
                __compression_type,
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            (),
        ),
    > {
        #[allow(clippy::used_underscore_binding, clippy::no_effect_underscore_binding)]
        pub fn compression_type(
            self,
            compression_type: impl ::core::convert::Into<
                Tag<'a, Text<'a>, CompressionType>,
            >,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                (Option<Tag<'a, Text<'a>, CompressionType>>,),
            ),
        > {
            let compression_type = (Some(compression_type.into()),);
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                (),
            ) = self.fields;
            SoapHeadersBuilder {
                fields: (
                    to,
                    action,
                    reply_to,
                    message_id,
                    relates_to,
                    resource_uri,
                    max_envelope_size,
                    locale,
                    data_locale,
                    session_id,
                    operation_id,
                    sequence_id,
                    option_set,
                    operation_timeout,
                    compression_type,
                ),
                phantom: self.phantom,
            }
        }
    }
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, non_snake_case)]
    #[allow(clippy::exhaustive_enums)]
    pub enum SoapHeadersBuilder_Error_Repeated_field_compression_type {}
    #[doc(hidden)]
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to,
        __action,
        __reply_to,
        __message_id,
        __relates_to,
        __resource_uri,
        __max_envelope_size,
        __locale,
        __data_locale,
        __session_id,
        __operation_id,
        __sequence_id,
        __option_set,
        __operation_timeout,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            (Option<Tag<'a, Text<'a>, CompressionType>>,),
        ),
    > {
        #[deprecated(note = "Repeated field compression_type")]
        pub fn compression_type(
            self,
            _: SoapHeadersBuilder_Error_Repeated_field_compression_type,
        ) -> SoapHeadersBuilder<
            'a,
            (
                __to,
                __action,
                __reply_to,
                __message_id,
                __relates_to,
                __resource_uri,
                __max_envelope_size,
                __locale,
                __data_locale,
                __session_id,
                __operation_id,
                __sequence_id,
                __option_set,
                __operation_timeout,
                (Option<Tag<'a, Text<'a>, CompressionType>>,),
            ),
        > {
            self
        }
    }
    #[allow(dead_code, non_camel_case_types, missing_docs)]
    #[automatically_derived]
    impl<
        'a,
        __to: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, To>>>,
        __action: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, Action>>>,
        __reply_to: ::typed_builder::Optional<
                Option<Tag<'a, AddressValue<'a>, ReplyTo>>,
            >,
        __message_id: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, MessageID>>>,
        __relates_to: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, RelatesTo>>>,
        __resource_uri: ::typed_builder::Optional<
                Option<Tag<'a, Text<'a>, ResourceURI>>,
            >,
        __max_envelope_size: ::typed_builder::Optional<
                Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
            >,
        __locale: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, Locale>>>,
        __data_locale: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, DataLocale>>>,
        __session_id: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, SessionId>>>,
        __operation_id: ::typed_builder::Optional<
                Option<Tag<'a, Text<'a>, OperationID>>,
            >,
        __sequence_id: ::typed_builder::Optional<Option<Tag<'a, Text<'a>, SequenceId>>>,
        __option_set: ::typed_builder::Optional<
                Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
            >,
        __operation_timeout: ::typed_builder::Optional<
                Option<Tag<'a, Text<'a>, OperationTimeout>>,
            >,
        __compression_type: ::typed_builder::Optional<
                Option<Tag<'a, Text<'a>, CompressionType>>,
            >,
    > SoapHeadersBuilder<
        'a,
        (
            __to,
            __action,
            __reply_to,
            __message_id,
            __relates_to,
            __resource_uri,
            __max_envelope_size,
            __locale,
            __data_locale,
            __session_id,
            __operation_id,
            __sequence_id,
            __option_set,
            __operation_timeout,
            __compression_type,
        ),
    > {
        #[allow(
            clippy::default_trait_access,
            clippy::used_underscore_binding,
            clippy::no_effect_underscore_binding
        )]
        pub fn build(self) -> SoapHeaders<'a> {
            let (
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            ) = self.fields;
            let to = ::typed_builder::Optional::into_value(
                to,
                || ::core::default::Default::default(),
            );
            let action = ::typed_builder::Optional::into_value(
                action,
                || ::core::default::Default::default(),
            );
            let reply_to = ::typed_builder::Optional::into_value(
                reply_to,
                || ::core::default::Default::default(),
            );
            let message_id = ::typed_builder::Optional::into_value(
                message_id,
                || ::core::default::Default::default(),
            );
            let relates_to = ::typed_builder::Optional::into_value(
                relates_to,
                || ::core::default::Default::default(),
            );
            let resource_uri = ::typed_builder::Optional::into_value(
                resource_uri,
                || ::core::default::Default::default(),
            );
            let max_envelope_size = ::typed_builder::Optional::into_value(
                max_envelope_size,
                || ::core::default::Default::default(),
            );
            let locale = ::typed_builder::Optional::into_value(
                locale,
                || ::core::default::Default::default(),
            );
            let data_locale = ::typed_builder::Optional::into_value(
                data_locale,
                || ::core::default::Default::default(),
            );
            let session_id = ::typed_builder::Optional::into_value(
                session_id,
                || ::core::default::Default::default(),
            );
            let operation_id = ::typed_builder::Optional::into_value(
                operation_id,
                || ::core::default::Default::default(),
            );
            let sequence_id = ::typed_builder::Optional::into_value(
                sequence_id,
                || ::core::default::Default::default(),
            );
            let option_set = ::typed_builder::Optional::into_value(
                option_set,
                || ::core::default::Default::default(),
            );
            let operation_timeout = ::typed_builder::Optional::into_value(
                operation_timeout,
                || ::core::default::Default::default(),
            );
            let compression_type = ::typed_builder::Optional::into_value(
                compression_type,
                || ::core::default::Default::default(),
            );
            #[allow(deprecated)]
            SoapHeaders::<'a> {
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            }
                .into()
        }
    }
    impl<'a> crate::cores::TagValue<'a> for SoapHeaders<'a> {
        fn append_to_element(
            self,
            element: xml::builder::Element<'a>,
        ) -> xml::builder::Element<'a> {
            let Self {
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            } = self;
            let mut array = Vec::new();
            if let Some(tag) = to {
                array.push(tag.into_element());
            }
            if let Some(tag) = action {
                array.push(tag.into_element());
            }
            if let Some(tag) = reply_to {
                array.push(tag.into_element());
            }
            if let Some(tag) = message_id {
                array.push(tag.into_element());
            }
            if let Some(tag) = relates_to {
                array.push(tag.into_element());
            }
            if let Some(tag) = resource_uri {
                array.push(tag.into_element());
            }
            if let Some(tag) = max_envelope_size {
                array.push(tag.into_element());
            }
            if let Some(tag) = locale {
                array.push(tag.into_element());
            }
            if let Some(tag) = data_locale {
                array.push(tag.into_element());
            }
            if let Some(tag) = session_id {
                array.push(tag.into_element());
            }
            if let Some(tag) = operation_id {
                array.push(tag.into_element());
            }
            if let Some(tag) = sequence_id {
                array.push(tag.into_element());
            }
            if let Some(tag) = option_set {
                array.push(tag.into_element());
            }
            if let Some(tag) = operation_timeout {
                array.push(tag.into_element());
            }
            if let Some(tag) = compression_type {
                array.push(tag.into_element());
            }
            element.add_children(array)
        }
    }
    pub struct SoapHeadersVisitor<'a> {
        pub to: Option<Tag<'a, Text<'a>, To>>,
        pub action: Option<Tag<'a, Text<'a>, Action>>,
        pub reply_to: Option<Tag<'a, AddressValue<'a>, ReplyTo>>,
        pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
        pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,
        pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
        pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
        pub locale: Option<Tag<'a, Text<'a>, Locale>>,
        pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
        pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
        pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
        pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
        pub option_set: Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
        pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
        pub compression_type: Option<Tag<'a, Text<'a>, CompressionType>>,
    }
    #[automatically_derived]
    impl<'a> ::core::fmt::Debug for SoapHeadersVisitor<'a> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "to",
                "action",
                "reply_to",
                "message_id",
                "relates_to",
                "resource_uri",
                "max_envelope_size",
                "locale",
                "data_locale",
                "session_id",
                "operation_id",
                "sequence_id",
                "option_set",
                "operation_timeout",
                "compression_type",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.to,
                &self.action,
                &self.reply_to,
                &self.message_id,
                &self.relates_to,
                &self.resource_uri,
                &self.max_envelope_size,
                &self.locale,
                &self.data_locale,
                &self.session_id,
                &self.operation_id,
                &self.sequence_id,
                &self.option_set,
                &self.operation_timeout,
                &&self.compression_type,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "SoapHeadersVisitor",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for SoapHeadersVisitor<'a> {
        #[inline]
        fn clone(&self) -> SoapHeadersVisitor<'a> {
            SoapHeadersVisitor {
                to: ::core::clone::Clone::clone(&self.to),
                action: ::core::clone::Clone::clone(&self.action),
                reply_to: ::core::clone::Clone::clone(&self.reply_to),
                message_id: ::core::clone::Clone::clone(&self.message_id),
                relates_to: ::core::clone::Clone::clone(&self.relates_to),
                resource_uri: ::core::clone::Clone::clone(&self.resource_uri),
                max_envelope_size: ::core::clone::Clone::clone(&self.max_envelope_size),
                locale: ::core::clone::Clone::clone(&self.locale),
                data_locale: ::core::clone::Clone::clone(&self.data_locale),
                session_id: ::core::clone::Clone::clone(&self.session_id),
                operation_id: ::core::clone::Clone::clone(&self.operation_id),
                sequence_id: ::core::clone::Clone::clone(&self.sequence_id),
                option_set: ::core::clone::Clone::clone(&self.option_set),
                operation_timeout: ::core::clone::Clone::clone(&self.operation_timeout),
                compression_type: ::core::clone::Clone::clone(&self.compression_type),
            }
        }
    }
    #[automatically_derived]
    impl<'a> ::core::default::Default for SoapHeadersVisitor<'a> {
        #[inline]
        fn default() -> SoapHeadersVisitor<'a> {
            SoapHeadersVisitor {
                to: ::core::default::Default::default(),
                action: ::core::default::Default::default(),
                reply_to: ::core::default::Default::default(),
                message_id: ::core::default::Default::default(),
                relates_to: ::core::default::Default::default(),
                resource_uri: ::core::default::Default::default(),
                max_envelope_size: ::core::default::Default::default(),
                locale: ::core::default::Default::default(),
                data_locale: ::core::default::Default::default(),
                session_id: ::core::default::Default::default(),
                operation_id: ::core::default::Default::default(),
                sequence_id: ::core::default::Default::default(),
                option_set: ::core::default::Default::default(),
                operation_timeout: ::core::default::Default::default(),
                compression_type: ::core::default::Default::default(),
            }
        }
    }
    impl<'a> xml::parser::XmlVisitor<'a> for SoapHeadersVisitor<'a> {
        type Value = SoapHeaders<'a>;
        fn visit_children(
            &mut self,
            children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
        ) -> Result<(), xml::XmlError<'a>> {
            for child in children {
                if !child.is_element() {
                    continue;
                }
                let tag_name = child.tag_name().name();
                let namespace = child.tag_name().namespace();
                (/*ERROR*/);
                match tag_name {
                    crate::cores::tag_name::To::TAG_NAME => {
                        (/*ERROR*/);
                        self.to = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::Action::TAG_NAME => {
                        (/*ERROR*/);
                        self.action = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::ReplyTo::TAG_NAME => {
                        (/*ERROR*/);
                        self.reply_to = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::MessageID::TAG_NAME => {
                        (/*ERROR*/);
                        self.message_id = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::RelatesTo::TAG_NAME => {
                        (/*ERROR*/);
                        self.relates_to = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::ResourceURI::TAG_NAME => {
                        (/*ERROR*/);
                        self.resource_uri = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::MaxEnvelopeSize::TAG_NAME => {
                        (/*ERROR*/);
                        self
                            .max_envelope_size = Some(
                            crate::cores::Tag::from_node(child)?,
                        );
                    }
                    crate::cores::tag_name::Locale::TAG_NAME => {
                        (/*ERROR*/);
                        self.locale = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::DataLocale::TAG_NAME => {
                        (/*ERROR*/);
                        self.data_locale = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::SessionId::TAG_NAME => {
                        (/*ERROR*/);
                        self.session_id = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::OperationID::TAG_NAME => {
                        (/*ERROR*/);
                        self.operation_id = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::SequenceId::TAG_NAME => {
                        (/*ERROR*/);
                        self.sequence_id = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::OptionSet::TAG_NAME => {
                        (/*ERROR*/);
                        self.option_set = Some(crate::cores::Tag::from_node(child)?);
                    }
                    crate::cores::tag_name::OperationTimeout::TAG_NAME => {
                        (/*ERROR*/);
                        self
                            .operation_timeout = Some(
                            crate::cores::Tag::from_node(child)?,
                        );
                    }
                    crate::cores::tag_name::CompressionType::TAG_NAME => {
                        (/*ERROR*/);
                        self
                            .compression_type = Some(
                            crate::cores::Tag::from_node(child)?,
                        );
                    }
                    _ => {
                        (/*ERROR*/);
                        return Err(
                            xml::XmlError::InvalidXml(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(
                                        format_args!(
                                            "Unknown tag in {0}: {1}",
                                            "SoapHeaders",
                                            tag_name,
                                        ),
                                    )
                                }),
                            ),
                        );
                    }
                }
            }
            Ok(())
        }
        fn visit_node(
            &mut self,
            node: xml::parser::Node<'a, 'a>,
        ) -> Result<(), xml::XmlError<'a>> {
            (/*ERROR*/);
            let children: Vec<_> = node.children().collect();
            (/*ERROR*/);
            self.visit_children(children.into_iter())?;
            Ok(())
        }
        fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
            let Self {
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            } = self;
            Ok(SoapHeaders {
                to,
                action,
                reply_to,
                message_id,
                relates_to,
                resource_uri,
                max_envelope_size,
                locale,
                data_locale,
                session_id,
                operation_id,
                sequence_id,
                option_set,
                operation_timeout,
                compression_type,
            })
        }
    }
    impl<'a> xml::parser::XmlDeserialize<'a> for SoapHeaders<'a> {
        type Visitor = SoapHeadersVisitor<'a>;
        fn visitor() -> Self::Visitor {
            SoapHeadersVisitor::default()
        }
        fn from_node(
            node: xml::parser::Node<'a, 'a>,
        ) -> Result<Self, xml::XmlError<'a>> {
            xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
        }
    }
}
