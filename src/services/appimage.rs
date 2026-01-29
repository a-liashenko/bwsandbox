use crate::{
    error::AppError,
    services::{BwrapInfo, Context, HandleType, Scope, Service},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppImageExtract {
    #[serde(default = "extract_and_run_default")]
    extract_and_run: bool,
}

fn extract_and_run_default() -> bool {
    true
}

impl AppImageExtract {
    // Keep consistent across services
    #[allow(clippy::unnecessary_wraps)]
    pub fn from_config(config: Self) -> Result<Self, AppError> {
        Ok(config)
    }
}

impl<C: Context> Service<C> for AppImageExtract {
    fn apply_before(&mut self, _: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        // Should work for appimage v2
        // https://github.com/AppImage/AppImageKit/issues/841
        if self.extract_and_run {
            ctx.command_mut()
                .arg("--setenv")
                .arg("APPIMAGE_EXTRACT_AND_RUN")
                .arg("1");
        }
        Ok(Scope::new())
    }

    fn start(self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        Ok(HandleType::None)
    }
}
