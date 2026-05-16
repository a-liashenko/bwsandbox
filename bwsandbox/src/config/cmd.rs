use crate::config::{ArgVal, Template};
use crate::error::AppError;
use serde::Deserialize;
use shlex::Shlex;
use std::ffi::OsString;

#[derive(Debug, Deserialize)]
pub struct Cmd {
    template: Option<Template>,
    inline: Option<Vec<ArgVal>>,
}

impl Cmd {
    fn iter_inline(&self) -> impl Iterator<Item = &ArgVal> {
        self.inline.iter().flat_map(|v| v.iter())
    }

    fn iter_template(&self) -> Result<TemplateArgs, AppError> {
        let rendered = self.template.as_ref().map(Template::render).transpose()?;
        Ok(TemplateArgs::new(rendered))
    }

    pub fn collect_args(&self) -> Result<Vec<OsString>, AppError> {
        let inline = self.iter_inline();

        // Only inline args size_hint worth for pre-allocation, shlex has blanket implementation
        let mut items = Vec::with_capacity(inline.size_hint().0);
        items.extend(inline.map(OsString::from));

        let template = self.iter_template()?;
        if let Some(mut iter) = template.iter() {
            items.extend(iter.by_ref().map(OsString::from));
            if iter.had_error {
                return Err(AppError::TemplateShlex);
            }
        }

        Ok(items)
    }
}

#[derive(Debug)]
pub struct TemplateArgs {
    rendered: Option<String>,
}

impl TemplateArgs {
    pub fn new(rendered: Option<String>) -> Self {
        Self { rendered }
    }

    pub fn iter(&'_ self) -> Option<Shlex<'_>> {
        self.rendered.as_deref().map(Shlex::new)
    }
}
