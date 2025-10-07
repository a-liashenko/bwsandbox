use crate::config::{ArgVal, Template, values::EnvVal};
use crate::error::AppError;
use serde::Deserialize;
use std::ffi::{OsStr, OsString};
use std::process::Command;

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
        let rendered = self.template.as_ref().map(|v| v.render()).transpose()?;
        Ok(TemplateArgs { rendered })
    }

    pub fn into_command(self, def_bin: impl AsRef<OsStr>) -> Result<Command, AppError> {
        let bin = self.bin().unwrap_or(def_bin.as_ref());
        let mut command = Command::new(bin);

        let template_args = self.iter_template()?;
        command.args(self.iter_inline()).args(template_args.iter());
        Ok(command)
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

impl<'a> std::iter::Iterator for TemplateArgsIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.shlex.as_mut().and_then(|v| v.next())
    }
}
