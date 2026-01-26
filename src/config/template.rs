use crate::config::{ArgVal, values::EnvVal};
use crate::error::AppError;
use minijinja::{Environment, UndefinedBehavior};
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Template {
    pub name: EnvVal<String>,
    pub dir: EnvVal<PathBuf>,
    #[serde(default)]
    pub context: BTreeMap<String, ArgVal>,
}

impl Template {
    pub fn render(&self) -> Result<String, AppError> {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        env.set_loader(minijinja::path_loader(self.dir.as_inner()));

        let template = env.get_template(self.name.as_inner())?;
        let content = template.render(&self.context)?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_parser() {
        let name = "template_name";
        let dir = "$HOME/template/dir";
        let one = "value_one";
        let two = "value_two";

        let config = toml::toml! {
            name = name
            dir = dir

            [context]
            key_one = { type = "str", value = one }
            key_two = { type = "str", value = two }
        };

        let v = toml::to_string_pretty(&config).unwrap();
        let v: Template = toml::from_str(&v).unwrap();
        assert_ne!(v.dir.as_inner(), PathBuf::from(dir).as_path());
        assert_eq!(v.name.as_inner(), "template_name");
        assert_eq!(v.context["key_one"], ArgVal::Str { value: one.into() });
        assert_eq!(v.context["key_two"], ArgVal::Str { value: two.into() });
    }
}
