// enum scmp_filter_attr {
// 	_SCMP_FLTATR_MIN = 0,
// 	SCMP_FLTATR_ACT_DEFAULT = 1,	/**< default filter action */
// 	SCMP_FLTATR_ACT_BADARCH = 2,	/**< bad architecture action */
// 	SCMP_FLTATR_CTL_NNP = 3,	/**< set NO_NEW_PRIVS on filter load */
// 	SCMP_FLTATR_CTL_TSYNC = 4,	/**< sync threads on filter load */
// 	SCMP_FLTATR_API_TSKIP = 5,	/**< allow rules with a -1 syscall */
// 	SCMP_FLTATR_CTL_LOG = 6,	/**< log not-allowed actions */
// 	SCMP_FLTATR_CTL_SSB = 7,	/**< disable SSB mitigation */
// 	SCMP_FLTATR_CTL_OPTIMIZE = 8,	/**< filter optimization level:
// 					 * 0 - currently unused
// 					 * 1 - rules weighted by priority and
// 					 *     complexity (DEFAULT)
// 					 * 2 - binary tree sorted by syscall
// 					 *     number
// 					 */
// 	SCMP_FLTATR_API_SYSRAWRC = 9,	/**< return the system return codes */
// 	SCMP_FLTATR_CTL_WAITKILL = 10,	/**< request wait killable semantics */
// 	_SCMP_FLTATR_MAX,
// };

use serde::{Deserialize, Serialize};
use std::ffi::c_int;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum FilterAttr {
    CtlOptimize = 8,
}

impl FilterAttr {
    pub fn raw(self) -> c_int {
        self as _
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FilterAttrOptimize {
    PriorityAndComplexity = 1,
    BinaryTree = 2,
}

impl FilterAttrOptimize {
    pub fn raw(self) -> u32 {
        self as _
    }
}
