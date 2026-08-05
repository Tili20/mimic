// Модель данных: glib-объект MimeType.

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use glib::prelude::*;
    use glib::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MimeType)]
    pub struct MimeType {
        #[property(get, set)]
        pub mime: RefCell<String>,
        #[property(get, set)]
        pub description: RefCell<String>,
        #[property(get, set)]
        pub category: RefCell<String>,
        #[property(get, set)]
        pub default_app: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MimeType {
        const NAME: &'static str = "MimicMimeType";
        type Type = super::MimeType;
    }

    impl ObjectImpl for MimeType {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub struct MimeType(ObjectSubclass<imp::MimeType>);
}

impl MimeType {
    pub fn new(
        mime: &str,
        description: &str,
        category: &str,
        default_app: Option<String>,
    ) -> Self {
        glib::Object::builder()
            .property("mime", mime)
            .property("description", description)
            .property("category", category)
            .property("default-app", default_app)
            .build()
    }
}
