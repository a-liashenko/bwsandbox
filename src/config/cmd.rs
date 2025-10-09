use crate::config::{ArgVal, Template, values::EnvVal};
use crate::error::AppError;
use serde::Deserialize;
use std::ffi::{OsStr, OsString};

#[derive(Debug, Deserialize)]
pub struct Cmd {
    bin: Option<EnvVal<OsString>>,
    template: Option<Template>,
    inline: Option<Vec<ArgVal>>,
}

impl Cmd {
    pub fn bin(&self) -> Option<&OsStr> {
        self.bin.as_ref().map(|v| v.as_inner().as_os_str())
    }

    pub fn iter_inline(&self) -> impl Iterator<Item = &ArgVal> {
        self.inline.iter().flat_map(|v| v.iter())
    }

    pub fn iter_template(&self) -> Result<TemplateArgs, AppError> {
        let rendered = self.template.as_ref().map(Template::render).transpose()?;
        Ok(TemplateArgs { rendered })
    }

    pub fn collect_args(&self) -> Result<Vec<OsString>, AppError> {
        let inline = self.iter_inline();
        let template = self.iter_template()?;

        let size = inline.size_hint().0 + template.iter().size_hint().0;
        let mut items = Vec::with_capacity(size);
        items.extend(inline.map(Into::into));
        items.extend(template.iter().map(Into::into));
        Ok(items)
    }
}

#[derive(Debug)]
pub struct TemplateArgs {
    rendered: Option<String>,
}

impl TemplateArgs {
    pub fn iter(&'_ self) -> TemplateArgsIter<'_> {
        match self.rendered.as_ref() {
            Some(v) => TemplateArgsIter::new(v),
            None => TemplateArgsIter::empty(),
        }
    }
}

pub struct TemplateArgsIter<'a> {
    shlex: Option<shlex::Shlex<'a>>,
}

impl<'a> TemplateArgsIter<'a> {
    fn new(source: &'a str) -> Self {
        let shlex = Some(shlex::Shlex::new(source));
        Self { shlex }
    }

    fn empty() -> Self {
        Self { shlex: None }
    }
}

impl std::iter::Iterator for TemplateArgsIter<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.shlex.as_mut().and_then(std::iter::Iterator::next)
    }
}
