use std::num::NonZeroU32;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingsCmdMsg {
    UpdateStrictLimits(StrictLimitsMsg),
}

impl SettingsCmdMsg {
    pub fn promote(
        self,
        _data: &collomatique_state_colloscopes::Data,
    ) -> crate::ops::SettingsUpdateOp {
        use crate::ops::SettingsUpdateOp;
        match self {
            SettingsCmdMsg::UpdateStrictLimits(strict_limits) => {
                SettingsUpdateOp::UpdateStrictLimits(strict_limits.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictLimitsMsg {
    pub interrogations_per_week: Option<std::ops::Range<u32>>,
    pub max_interrogations_per_day: Option<NonZeroU32>,
}

impl From<StrictLimitsMsg> for collomatique_state_colloscopes::settings::StrictLimits {
    fn from(value: StrictLimitsMsg) -> Self {
        use collomatique_state_colloscopes::settings::StrictLimits;
        StrictLimits {
            interrogations_per_week: value.interrogations_per_week,
            max_interrogations_per_day: value.max_interrogations_per_day,
        }
    }
}
